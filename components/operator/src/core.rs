use std::{cmp::Ordering, collections::BTreeMap, collections::HashMap, str::FromStr};

use anyhow::{anyhow, ensure, Context, Result};
use cosmwasm_std::{Decimal256, Timestamp, Uint128, Uint256};
use layer_climb::{prelude::Address, querier::QueryClient};
use serde::{Deserialize, Serialize};
use vault::{
    Payload, QueryMsg, SwapOperation as VaultSwapOperation, SwapRoute, VaultQueryMsg, VaultState,
};

use crate::{
    coingecko::{get_neutron_asset, CoinGeckoApiClient},
    skip::SkipAPIClient,
};
use vault::PriceInfo;

#[derive(Serialize, Deserialize)]
pub enum TradeStrategy {
    AI, // Placeholder for now
    Fixed(BTreeMap<String, Decimal256>),
}

impl TradeStrategy {
    pub fn validate(&self) -> Result<()> {
        match self {
            TradeStrategy::AI => {}
            TradeStrategy::Fixed(map) => {
                let mut total = Decimal256::zero();
                for allocation in map.values() {
                    total = total.checked_add(*allocation)?;
                }

                ensure!(
                    total == Decimal256::one(),
                    "Total fixed allocation must be equal to one"
                )
            }
        };

        Ok(())
    }
}

pub async fn generate_payload(
    query_client: QueryClient,
    addr: Address,
    trade_strategy: TradeStrategy,
    timestamp: u64,
    chain_id: String,
) -> Result<Payload> {
    trade_strategy.validate()?;

    let vault_state: VaultState = query_client
        .contract_smart(&addr, &QueryMsg::Vault(VaultQueryMsg::GetVaultState {}))
        .await
        .context("failed to query vault state")?;

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
        let fresh_prices = coingecko_client.query_prices(&asset_queries, "usd").await?;
        apply_fresh_prices(&mut price_map, fresh_prices);
    }

    let holdings: HashMap<String, Uint256> = funds
        .iter()
        .map(|coin| (coin.denom.clone(), coin.amount))
        .collect();

    let mut surplus_list: Vec<HoldingSurplus> = Vec::new();
    let mut deficit_list: Vec<HoldingDeficit> = Vec::new();

    for (denom, amount) in &holdings {
        let price = match price_map.get(denom) {
            Some(price) if !price.is_zero() => *price,
            _ => continue,
        };

        let amount_decimal = Decimal256::from_atomics(*amount, 0)
            .map_err(|e| anyhow!("failed to convert holdings to decimal: {e}"))?;
        let current_value = price
            .checked_mul(amount_decimal)
            .map_err(|e| anyhow!("overflow while evaluating holdings value: {e}"))?;
        let target_value = allocation_targets
            .get(denom)
            .copied()
            .unwrap_or_else(Decimal256::zero);

        match current_value.cmp(&target_value) {
            Ordering::Greater => {
                let delta = current_value
                    .checked_sub(target_value)
                    .map_err(|e| anyhow!("overflow while computing surplus: {e}"))?;
                surplus_list.push(HoldingSurplus {
                    denom: denom.clone(),
                    amount: *amount,
                    price,
                    usd_surplus: delta,
                });
            }
            Ordering::Less => {
                let delta = target_value
                    .checked_sub(current_value)
                    .map_err(|e| anyhow!("overflow while computing deficit: {e}"))?;
                deficit_list.push(HoldingDeficit {
                    denom: denom.clone(),
                    usd_deficit: delta,
                });
            }
            Ordering::Equal => {}
        }
    }

    for (denom, target_value) in &allocation_targets {
        if holdings.contains_key(denom) {
            continue;
        }
        if target_value.is_zero() {
            continue;
        }
        if price_map.get(denom).is_some_and(|price| !price.is_zero()) {
            deficit_list.push(HoldingDeficit {
                denom: denom.clone(),
                usd_deficit: *target_value,
            });
        }
    }

    let skip_client = SkipAPIClient::new(chain_id);
    let mut swap_routes_vec: Vec<SwapRoute> = Vec::new();

    while !surplus_list.is_empty() && !deficit_list.is_empty() {
        surplus_list.sort_by(|a, b| b.usd_surplus.cmp(&a.usd_surplus));
        deficit_list.sort_by(|a, b| b.usd_deficit.cmp(&a.usd_deficit));

        let mut surplus = surplus_list.remove(0);
        let mut deficit = deficit_list.remove(0);

        let Some(plan) = build_swap_route(&surplus, &deficit, &skip_client, timestamp).await?
        else {
            break;
        };

        surplus.usd_surplus = surplus
            .usd_surplus
            .checked_sub(plan.usd_used)
            .map_err(|e| anyhow!("failed to update surplus allocation: {e}"))?;
        surplus.amount = surplus
            .amount
            .checked_sub(plan.amount_in)
            .map_err(|e| anyhow!("failed to update surplus amount: {e}"))?;
        deficit.usd_deficit = deficit
            .usd_deficit
            .checked_sub(plan.usd_used)
            .map_err(|e| anyhow!("failed to update deficit allocation: {e}"))?;

        swap_routes_vec.push(plan.route);

        if !surplus.usd_surplus.is_zero() && !surplus.amount.is_zero() {
            surplus_list.push(surplus);
        }
        if !deficit.usd_deficit.is_zero() {
            deficit_list.push(deficit);
        }
    }

    let swap_routes = if swap_routes_vec.is_empty() {
        None
    } else {
        Some(swap_routes_vec)
    };

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
    usd_surplus: Decimal256,
}

struct HoldingDeficit {
    denom: String,
    usd_deficit: Decimal256,
}

struct SwapPlan {
    route: SwapRoute,
    usd_used: Decimal256,
    amount_in: Uint256,
}

async fn build_swap_route(
    surplus: &HoldingSurplus,
    deficit: &HoldingDeficit,
    skip_client: &SkipAPIClient,
    timestamp: u64,
) -> Result<Option<SwapPlan>> {
    if surplus.usd_surplus.is_zero() || deficit.usd_deficit.is_zero() {
        return Ok(None);
    }

    let surplus_amount_decimal = Decimal256::from_atomics(surplus.amount, 0)
        .map_err(|e| anyhow!("failed to convert surplus token balance to Decimal256: {e}"))?;
    let available_sell_usd = surplus
        .price
        .checked_mul(surplus_amount_decimal)
        .map_err(|e| anyhow!("overflow calculating available sell usd: {e}"))?;

    let mut trade_usd = min_decimal(surplus.usd_surplus, deficit.usd_deficit);
    trade_usd = min_decimal(trade_usd, available_sell_usd);

    if trade_usd.is_zero() {
        return Ok(None);
    }

    let amount_in_decimal = trade_usd
        .checked_div(surplus.price)
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
        usd_used: trade_usd,
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
