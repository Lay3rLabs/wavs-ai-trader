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
        host::LogLevel::Debug,
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
        pending_assets: _,
        prices,
        tvl,
    } = vault_state;

    let mut price_map: HashMap<String, Decimal256> = HashMap::new();
    for price in &prices {
        price_map.insert(price.denom.clone(), price.price_usd);
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
            host::LogLevel::Debug,
            &format!("Retrieved {} fresh prices", fresh_prices.len()),
        );
        apply_fresh_prices(&mut price_map, fresh_prices);
    } else {
        host::log(host::LogLevel::Debug, "No external price queries needed");
    }

    let holdings: HashMap<String, Uint256> = funds
        .iter()
        .map(|coin| (coin.denom.clone(), coin.amount))
        .collect();

    let mut surplus_list: Vec<HoldingSurplus> = Vec::new();
    let mut deficit_list: Vec<HoldingDeficit> = Vec::new();

    let mut denominators: HashSet<String> = holdings.keys().cloned().collect();
    denominators.extend(allocation_targets.keys().cloned());

    host::log(
        host::LogLevel::Debug,
        &format!("Analyzing {} assets for rebalancing", denominators.len()),
    );

    for denom in denominators {
        let amount = holdings.get(&denom).copied().unwrap_or_else(Uint256::zero);
        let price_option = price_map.get(&denom).copied();
        let target_value = allocation_targets
            .get(&denom)
            .copied()
            .unwrap_or_else(Decimal256::zero);

        let price = match price_option {
            Some(price) if !price.is_zero() => price,
            _ if amount.is_zero() => {
                // No holdings and missing price: we can still consider deficits using the target allocation.
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

        // Scale the price for base unit calculations
        let scaled_price = if let Some((_, decimals)) = get_neutron_asset(&denom) {
            let scale =
                Decimal256::from_ratio(Uint256::one(), Uint256::from(10u128.pow(decimals as u32)));
            price
                .checked_mul(scale)
                .map_err(|e| anyhow!("overflow scaling price: {e}"))?
        } else {
            price
        };

        let amount_decimal = Decimal256::from_atomics(amount, 0)
            .map_err(|e| anyhow!("failed to convert holdings to decimal: {e}"))?;
        let current_value = scaled_price
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
                    price,
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

struct HoldingSurplus {
    denom: String,
    amount: Uint256,
    price: Decimal256,
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

    let surplus_amount_decimal = Decimal256::from_atomics(surplus.amount, 0)
        .map_err(|e| anyhow!("failed to convert surplus token balance to Decimal256: {e}"))?;

    // Scale the price for base unit calculations
    let scaled_price = if let Some((_, decimals)) = get_neutron_asset(&surplus.denom) {
        let scale =
            Decimal256::from_ratio(Uint256::one(), Uint256::from(10u128.pow(decimals as u32)));
        surplus
            .price
            .checked_mul(scale)
            .map_err(|e| anyhow!("overflow scaling surplus price: {e}"))?
    } else {
        surplus.price
    };

    let available_sell_usd = scaled_price
        .checked_mul(surplus_amount_decimal)
        .map_err(|e| anyhow!("overflow calculating available sell usd: {e}"))?;

    let trade_usd = min_decimal(usd_to_trade, available_sell_usd);

    if trade_usd.is_zero() {
        return Ok(None);
    }

    let amount_in_decimal = trade_usd
        .checked_div(scaled_price)
        .map_err(|e| anyhow!("overflow calculating amount in: {e}"))?;
    let mut trade_amount_uint256 = amount_in_decimal.to_uint_floor();
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

    let actual_amount_decimal = Decimal256::from_atomics(trade_amount_uint256, 0)
        .map_err(|e| anyhow!("failed to convert trade amount to decimal: {e}"))?;
    let usd_used = scaled_price
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

fn price_lookup_assets(price_map: &HashMap<String, Decimal256>) -> Vec<(String, String, u8)> {
    price_map
        .keys()
        .filter_map(|denom| {
            get_neutron_asset(denom).map(|(id, decimals)| (denom.clone(), id, decimals))
        })
        .collect()
}

fn apply_fresh_prices(
    price_map: &mut HashMap<String, Decimal256>,
    fresh: HashMap<String, Decimal256>,
) {
    for (denom, price) in fresh {
        price_map.insert(denom, price);
    }
}

fn to_price_info(price_map: &HashMap<String, Decimal256>) -> Vec<PriceInfo> {
    price_map
        .iter()
        .map(|(denom, price)| PriceInfo {
            denom: denom.clone(),
            price_usd: *price,
        })
        .collect()
}
