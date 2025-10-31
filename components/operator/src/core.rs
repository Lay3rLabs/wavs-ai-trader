use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use ai_portfolio_types::TradeStrategy;
use anyhow::{anyhow, Context, Result};
use cosmwasm_std::{Decimal256, Timestamp, Uint128, Uint256};
use layer_climb::{prelude::Address, querier::QueryClient};
use vault::{
    Payload, QueryMsg, SwapOperation as VaultSwapOperation, SwapRoute, VaultQueryMsg, VaultState,
};

use crate::host;
use crate::{
    coingecko::{get_neutron_asset, CoinGeckoApiClient},
    skip::SkipAPIClient,
};
use vault::PriceInfo;

pub async fn generate_payload(
    query_client: QueryClient,
    addr: Address,
    trade_strategy: TradeStrategy,
    timestamp: u64,
    chain_id: String,
) -> Result<Payload> {
    host::log(
        host::LogLevel::Info,
        &format!("Starting payload generation for vault: {}", addr),
    );

    trade_strategy.validate()?;

    host::log(
        host::LogLevel::Info,
        "Trade strategy validated successfully",
    );

    let vault_state: VaultState = query_client
        .contract_smart(&addr, &QueryMsg::Vault(VaultQueryMsg::GetVaultState {}))
        .await
        .context("failed to query vault state")?;

    host::log(
        host::LogLevel::Info,
        &format!(
            "Vault state retrieved: TVL={}, holdings={}",
            vault_state.tvl,
            vault_state.funds.len()
        ),
    );

    let VaultState {
        funds,
        total_pending_assets: _,
        prices,
        tvl,
    } = vault_state;

    let mut price_map: HashMap<String, AssetPrice> = HashMap::new();
    for price in &prices {
        price_map.insert(
            price.denom.clone(),
            AssetPrice {
                display_price: price.price_usd,
                decimals: price.decimals,
            },
        );
    }

    let allocation_targets = match &trade_strategy {
        TradeStrategy::AI => unimplemented!(),
        TradeStrategy::Fixed(map) => {
            let mut targets = HashMap::new();
            for (denom, allocation) in map {
                let target_value = tvl
                    .checked_mul(*allocation)
                    .context("overflow while calculating allocation target")?;
                targets.insert(denom.clone(), target_value);
            }
            targets
        }
    };

    let coingecko_client =
        CoinGeckoApiClient::new(std::env::var("WAVS_ENV_COINGECKO_API_KEY").ok());
    let asset_queries = price_lookup_assets(&price_map);
    if !asset_queries.is_empty() {
        host::log(
            host::LogLevel::Info,
            &format!(
                "Querying prices for {} assets from CoinGecko",
                asset_queries.len()
            ),
        );
        let fresh_prices = coingecko_client.query_prices(&asset_queries, "usd").await?;
        host::log(
            host::LogLevel::Info,
            &format!("Retrieved {} fresh prices", fresh_prices.len()),
        );
        apply_fresh_prices(&mut price_map, fresh_prices);
    } else {
        host::log(host::LogLevel::Info, "No external price queries needed");
    }

    let holdings: HashMap<String, Uint256> = funds
        .iter()
        .map(|coin| (coin.denom.clone(), coin.amount))
        .collect();

    let mut denominators: HashSet<String> = holdings.keys().cloned().collect();
    denominators.extend(allocation_targets.keys().cloned());

    host::log(
        host::LogLevel::Info,
        &format!("Analyzing {} assets for rebalancing", denominators.len()),
    );

    let (mut surplus_list, mut deficit_list) = analyze_positions(
        denominators,
        &holdings,
        &price_map,
        &allocation_targets,
        tvl,
    )?;

    let skip_client = SkipAPIClient::new(chain_id);
    let mut swap_routes_vec: Vec<SwapRoute> = Vec::new();

    host::log(
        host::LogLevel::Info,
        &format!(
            "Identified {} surplus assets and {} deficit assets",
            surplus_list.len(),
            deficit_list.len()
        ),
    );

    if !surplus_list.is_empty() && !deficit_list.is_empty() {
        for deficit in &mut deficit_list {
            while !deficit.usd_remaining.is_zero() {
                let mut total_surplus = sum_decimal(surplus_list.iter().map(|s| s.usd_remaining))?;
                if total_surplus.is_zero() {
                    break;
                }

                let mut progressed = false;

                for surplus in &mut surplus_list {
                    if surplus.usd_remaining.is_zero() || surplus.amount.is_zero() {
                        continue;
                    }

                    if total_surplus.is_zero() {
                        break;
                    }

                    let share = surplus
                        .usd_remaining
                        .checked_div(total_surplus)
                        .map_err(|e| anyhow!("overflow while computing surplus share: {e}"))?;
                    let mut usd_to_trade =
                        deficit.usd_remaining.checked_mul(share).map_err(|e| {
                            anyhow!("overflow while distributing deficit allocation: {e}")
                        })?;

                    usd_to_trade = min_decimal(usd_to_trade, surplus.usd_remaining);
                    usd_to_trade = min_decimal(usd_to_trade, deficit.usd_remaining);

                    if usd_to_trade.is_zero() {
                        continue;
                    }

                    let Some(plan) =
                        build_swap_route(surplus, deficit, usd_to_trade, &skip_client, timestamp)
                            .await?
                    else {
                        continue;
                    };

                    if plan.usd_used.is_zero() {
                        continue;
                    }

                    surplus.usd_remaining = surplus
                        .usd_remaining
                        .checked_sub(plan.usd_used)
                        .map_err(|e| anyhow!("failed to update surplus allocation: {e}"))?;
                    surplus.amount = surplus
                        .amount
                        .checked_sub(plan.amount_in)
                        .map_err(|e| anyhow!("failed to update surplus amount: {e}"))?;
                    deficit.usd_remaining = deficit
                        .usd_remaining
                        .checked_sub(plan.usd_used)
                        .map_err(|e| anyhow!("failed to update deficit allocation: {e}"))?;
                    total_surplus = total_surplus
                        .checked_sub(plan.usd_used)
                        .map_err(|e| anyhow!("failed to update total surplus tracker: {e}"))?;

                    swap_routes_vec.push(plan.route);
                    progressed = true;

                    if deficit.usd_remaining.is_zero() {
                        break;
                    }
                }

                if !progressed {
                    break;
                }
            }
        }
    }

    let swap_routes = if swap_routes_vec.is_empty() {
        None
    } else {
        Some(swap_routes_vec.clone())
    };

    host::log(
        host::LogLevel::Info,
        &format!(
            "Payload generation complete: {} swap routes planned",
            swap_routes_vec.len()
        ),
    );

    Ok(Payload {
        timestamp: Timestamp::from_nanos(timestamp),
        prices: to_price_info(&price_map),
        swap_routes,
    })
}

#[derive(Clone)]
struct AssetPrice {
    display_price: Decimal256,
    decimals: u8,
}

struct HoldingSurplus {
    denom: String,
    amount: Uint256,
    price: Decimal256,
    decimals: u8,
    usd_remaining: Decimal256,
}

struct HoldingDeficit {
    denom: String,
    usd_remaining: Decimal256,
}

struct SwapPlan {
    route: SwapRoute,
    usd_used: Decimal256,
    amount_in: Uint256,
}

fn analyze_positions(
    denominators: HashSet<String>,
    holdings: &HashMap<String, Uint256>,
    price_map: &HashMap<String, AssetPrice>,
    allocation_targets: &HashMap<String, Decimal256>,
    tvl: Decimal256,
) -> Result<(Vec<HoldingSurplus>, Vec<HoldingDeficit>)> {
    let mut surplus_list: Vec<HoldingSurplus> = Vec::new();
    let mut deficit_list: Vec<HoldingDeficit> = Vec::new();

    for denom in denominators {
        let amount = holdings.get(&denom).copied().unwrap_or_else(Uint256::zero);
        let price_option = price_map.get(&denom);
        let target_value = allocation_targets
            .get(&denom)
            .copied()
            .unwrap_or_else(Decimal256::zero);

        let price_entry = match price_option {
            Some(entry) if !entry.display_price.is_zero() => entry,
            _ if amount.is_zero() => {
                if target_value.is_zero() {
                    continue;
                }

                if exceeds_tolerance(target_value, tvl) {
                    deficit_list.push(HoldingDeficit {
                        denom,
                        usd_remaining: target_value,
                    });
                }
                continue;
            }
            _ => continue,
        };

        let decimals = u32::from(price_entry.decimals);
        let amount_decimal = Decimal256::from_atomics(amount, decimals)
            .map_err(|e| anyhow!("failed to convert holdings to decimal: {e}"))?;
        let current_value = price_entry
            .display_price
            .checked_mul(amount_decimal)
            .map_err(|e| anyhow!("overflow while evaluating holdings value: {e}"))?;

        if current_value > target_value {
            let delta = current_value
                .checked_sub(target_value)
                .map_err(|e| anyhow!("overflow while computing surplus: {e}"))?;

            if exceeds_tolerance(delta, tvl) {
                surplus_list.push(HoldingSurplus {
                    denom,
                    amount,
                    price: price_entry.display_price,
                    decimals: price_entry.decimals,
                    usd_remaining: delta,
                });
            }
        } else if current_value < target_value {
            let delta = target_value
                .checked_sub(current_value)
                .map_err(|e| anyhow!("overflow while computing deficit: {e}"))?;

            if exceeds_tolerance(delta, tvl) {
                deficit_list.push(HoldingDeficit {
                    denom,
                    usd_remaining: delta,
                });
            }
        }
    }

    Ok((surplus_list, deficit_list))
}

async fn build_swap_route(
    surplus: &HoldingSurplus,
    deficit: &HoldingDeficit,
    usd_to_trade: Decimal256,
    skip_client: &SkipAPIClient,
    timestamp: u64,
) -> Result<Option<SwapPlan>> {
    if usd_to_trade.is_zero() {
        return Ok(None);
    }

    let decimals_u32 = u32::from(surplus.decimals);
    let surplus_amount_decimal = Decimal256::from_atomics(surplus.amount, decimals_u32)
        .map_err(|e| anyhow!("failed to convert surplus token balance to Decimal256: {e}"))?;

    let available_sell_usd = surplus
        .price
        .checked_mul(surplus_amount_decimal)
        .map_err(|e| anyhow!("overflow calculating available sell usd: {e}"))?;

    let trade_usd = min_decimal(usd_to_trade, available_sell_usd);

    if trade_usd.is_zero() {
        return Ok(None);
    }

    let display_amount = trade_usd
        .checked_div(surplus.price)
        .map_err(|e| anyhow!("overflow calculating amount in: {e}"))?;

    let mut trade_amount_uint256 = if surplus.decimals > 0 {
        let pow10 = 10u128
            .checked_pow(u32::from(surplus.decimals))
            .ok_or_else(|| anyhow!("unsupported decimal precision: {}", surplus.decimals))?;
        let scale = Decimal256::from_ratio(Uint256::from(pow10), Uint256::one());
        display_amount
            .checked_mul(scale)
            .map_err(|e| anyhow!("overflow scaling amount in to base units: {e}"))?
            .to_uint_floor()
    } else {
        display_amount.to_uint_floor()
    };
    if trade_amount_uint256.is_zero() {
        return Ok(None);
    }

    if trade_amount_uint256 > surplus.amount {
        trade_amount_uint256 = surplus.amount;
    }

    let max_supported = Uint256::from(Uint128::MAX);
    if trade_amount_uint256 > max_supported {
        trade_amount_uint256 = max_supported;
    }

    if trade_amount_uint256.is_zero() {
        return Ok(None);
    }

    let amount_in = Uint128::from_str(&trade_amount_uint256.to_string())
        .map_err(|e| anyhow!("amount in exceeds supported range: {e}"))?;
    if amount_in.is_zero() {
        return Ok(None);
    }

    let actual_amount_decimal = Decimal256::from_atomics(trade_amount_uint256, decimals_u32)
        .map_err(|e| anyhow!("failed to convert trade amount to decimal: {e}"))?;
    let usd_used = surplus
        .price
        .checked_mul(actual_amount_decimal)
        .map_err(|e| anyhow!("overflow calculating usd used: {e}"))?;

    if usd_used.is_zero() {
        return Ok(None);
    }

    let route_plan = skip_client
        .plan_route(&surplus.denom, &deficit.denom, amount_in)
        .await?;

    if !route_plan.does_swap || route_plan.source_asset_chain_id != route_plan.dest_asset_chain_id {
        return Ok(None);
    }

    let mut operations: Vec<VaultSwapOperation> = Vec::new();
    let mut venue_name: Option<String> = None;

    for op in &route_plan.operations {
        if let Some(swap) = &op.swap {
            venue_name = Some(swap.swap_in.swap_venue.name.clone());
            for swap_operation in &swap.swap_in.swap_operations {
                operations.push(VaultSwapOperation {
                    pool: swap_operation.pool.clone(),
                    denom_in: swap_operation.denom_in.clone(),
                    denom_out: swap_operation.denom_out.clone(),
                    interface: None,
                });
            }
        }
    }

    if operations.is_empty() {
        return Ok(None);
    }

    let estimated_amount_out = Uint128::from_str(&route_plan.estimated_amount_out)
        .map_err(|e| anyhow!("failed to parse estimated output amount: {e}"))?;
    if estimated_amount_out.is_zero() {
        return Ok(None);
    }

    let minimum_amount_out = estimated_amount_out.multiply_ratio(9_900u128, 10_000u128); // 1% slippage buffer

    let base_timestamp = Timestamp::from_nanos(timestamp);
    let timeout = base_timestamp.plus_seconds(600);

    let swap_route = SwapRoute {
        swap_venue_name: venue_name.unwrap_or_else(|| "unknown".to_string()),
        offer_denom: surplus.denom.clone(),
        ask_denom: deficit.denom.clone(),
        amount_in,
        estimated_amount_out,
        minimum_amount_out: Some(minimum_amount_out),
        timeout,
        operations,
    };

    Ok(Some(SwapPlan {
        route: swap_route,
        usd_used,
        amount_in: trade_amount_uint256,
    }))
}

fn min_decimal(left: Decimal256, right: Decimal256) -> Decimal256 {
    if left <= right {
        left
    } else {
        right
    }
}

fn sum_decimal<I>(mut iter: I) -> Result<Decimal256>
where
    I: Iterator<Item = Decimal256>,
{
    iter.try_fold(Decimal256::zero(), |acc, value| acc.checked_add(value))
        .map_err(|e| anyhow!("overflow while summing decimal values: {e}"))
}

fn exceeds_tolerance(delta: Decimal256, tvl: Decimal256) -> bool {
    if delta.is_zero() {
        return false;
    }

    if tvl.is_zero() {
        return true;
    }

    match delta.checked_div(tvl) {
        Ok(ratio) => ratio > rebalance_tolerance(),
        Err(_) => true,
    }
}

fn rebalance_tolerance() -> Decimal256 {
    Decimal256::from_ratio(5u128, 1000u128)
}

fn price_lookup_assets(price_map: &HashMap<String, AssetPrice>) -> Vec<(String, String, u8)> {
    price_map
        .keys()
        .filter_map(|denom| {
            get_neutron_asset(denom).map(|(id, decimals)| (denom.clone(), id, decimals))
        })
        .collect()
}

fn apply_fresh_prices(
    price_map: &mut HashMap<String, AssetPrice>,
    fresh: HashMap<String, Decimal256>,
) {
    for (denom, price) in fresh {
        let decimals_override = get_neutron_asset(&denom).map(|(_, d)| d);
        price_map
            .entry(denom.clone())
            .and_modify(|entry| {
                entry.display_price = price;
                if let Some(dec) = decimals_override {
                    entry.decimals = dec;
                }
            })
            .or_insert(AssetPrice {
                display_price: price,
                decimals: decimals_override.unwrap_or(0),
            });
    }
}

fn to_price_info(price_map: &HashMap<String, AssetPrice>) -> Vec<PriceInfo> {
    price_map
        .iter()
        .map(|(denom, asset_price)| PriceInfo {
            denom: denom.clone(),
            price_usd: asset_price.display_price,
            decimals: asset_price.decimals,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint256;
    use std::collections::{HashMap, HashSet};
    use std::str::FromStr;

    const DENOM_NTRN: &str = "untrn";
    const DENOM_USDC: &str = "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81";
    const DENOM_DYDX: &str = "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130";

    fn decimal(value: &str) -> Decimal256 {
        Decimal256::from_str(value).expect("valid decimal string")
    }

    fn asset_price(denom: &str, display_price: &str) -> AssetPrice {
        AssetPrice {
            display_price: decimal(display_price),
            decimals: get_neutron_asset(denom)
                .map(|(_, decimals)| decimals)
                .unwrap_or(0),
        }
    }

    #[test]
    fn asset_price_for_high_precision_assets_has_precision() {
        let price = asset_price(DENOM_DYDX, "0.310581");
        assert!(price.display_price > Decimal256::zero());
        assert_eq!(price.decimals, 18);
    }

    #[test]
    fn analyze_positions_with_display_tvl_has_no_surplus() {
        let mut holdings = HashMap::new();
        holdings.insert(DENOM_NTRN.to_string(), Uint256::from(2000u128));

        let mut prices = HashMap::new();
        prices.insert(
            DENOM_NTRN.to_string(),
            asset_price(DENOM_NTRN, "0.04371765"),
        );
        prices.insert(DENOM_USDC.to_string(), asset_price(DENOM_USDC, "0.999748"));

        let tvl = decimal("87.4353");

        let mut allocation_targets = HashMap::new();
        let half = decimal("43.71765");
        allocation_targets.insert(DENOM_NTRN.to_string(), half);
        allocation_targets.insert(DENOM_USDC.to_string(), half);

        let mut denominators: HashSet<String> = holdings.keys().cloned().collect();
        denominators.extend(allocation_targets.keys().cloned());

        let (surpluses, deficits) =
            analyze_positions(denominators, &holdings, &prices, &allocation_targets, tvl)
                .expect("analysis succeeds");

        assert!(surpluses.is_empty(), "unexpected surplus detected");
        assert_eq!(deficits.len(), 2, "expected both assets to be deficient");
        assert!(deficits.iter().any(|d| d.denom == DENOM_NTRN));
        assert!(deficits.iter().any(|d| d.denom == DENOM_USDC));
    }

    #[test]
    fn analyze_positions_with_base_tvl_detects_expected_surplus() {
        let mut holdings = HashMap::new();
        holdings.insert(DENOM_NTRN.to_string(), Uint256::from(2000u128));

        let mut prices = HashMap::new();
        prices.insert(
            DENOM_NTRN.to_string(),
            asset_price(DENOM_NTRN, "0.04371765"),
        );
        prices.insert(DENOM_USDC.to_string(), asset_price(DENOM_USDC, "0.999748"));

        let tvl = decimal("0.0000874353");

        let mut allocation_targets = HashMap::new();
        let half = decimal("0.00004371765");
        allocation_targets.insert(DENOM_NTRN.to_string(), half);
        allocation_targets.insert(DENOM_USDC.to_string(), half);

        let mut denominators: HashSet<String> = holdings.keys().cloned().collect();
        denominators.extend(allocation_targets.keys().cloned());

        let (surpluses, deficits) =
            analyze_positions(denominators, &holdings, &prices, &allocation_targets, tvl)
                .expect("analysis succeeds");

        assert_eq!(surpluses.len(), 1, "expected a single surplus asset");
        let ntrn_surplus = &surpluses[0];
        assert_eq!(ntrn_surplus.denom, DENOM_NTRN);
        assert_eq!(ntrn_surplus.usd_remaining, half);

        assert_eq!(deficits.len(), 1, "expected a single deficit asset");
        let usdc_deficit = &deficits[0];
        assert_eq!(usdc_deficit.denom, DENOM_USDC);
        assert_eq!(usdc_deficit.usd_remaining, half);
    }
}
