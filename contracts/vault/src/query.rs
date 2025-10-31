use cosmwasm_std::{Coin, Decimal256, Deps, Order, StdResult, Uint256};
use cw_storage_plus::Bound;

use crate::{
    state::{
        DEPOSIT_REQUESTS, PRICES, TOTAL_PENDING_ASSETS, TOTAL_SHARES, VAULT_ASSETS,
        VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
    },
    DepositRequest, PriceInfo, VaultState,
};

pub fn total_shares(deps: Deps) -> StdResult<Uint256> {
    TOTAL_SHARES.load(deps.storage)
}

pub fn vault_value(deps: Deps) -> StdResult<Decimal256> {
    VAULT_VALUE_DEPOSITED.load(deps.storage)
}

pub fn whitelisted_denoms(deps: Deps) -> StdResult<Vec<String>> {
    WHITELISTED_DENOMS
        .keys(deps.storage, None, None, Order::Ascending)
        .take(100) // Limit to 100 for safety
        .collect()
}

pub fn deposit_request(deps: Deps, deposit_id: u64) -> StdResult<DepositRequest> {
    DEPOSIT_REQUESTS.load(deps.storage, deposit_id)
}

pub fn deposit_requests(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<DepositRequest>> {
    let start = start_after.map(Bound::exclusive);
    DEPOSIT_REQUESTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit.unwrap_or(30) as usize)
        .map(|item| item.map(|(_, deposit)| deposit))
        .collect()
}

pub fn vault_assets(deps: Deps) -> StdResult<Vec<Coin>> {
    VAULT_ASSETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

pub fn vault_asset_balance(deps: Deps, denom: String) -> StdResult<Uint256> {
    VAULT_ASSETS
        .load(deps.storage, denom)
        .or(Ok(Uint256::zero()))
}

pub fn total_pending_assets(deps: Deps) -> StdResult<Vec<Coin>> {
    TOTAL_PENDING_ASSETS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

pub fn pending_asset_balance(deps: Deps, denom: String) -> StdResult<Uint256> {
    TOTAL_PENDING_ASSETS
        .load(deps.storage, denom)
        .or(Ok(Uint256::zero()))
}

pub fn price(deps: Deps, denom: String) -> StdResult<Decimal256> {
    PRICES.load(deps.storage, denom).or(Ok(Decimal256::zero()))
}

pub fn prices(deps: Deps) -> StdResult<Vec<PriceInfo>> {
    PRICES
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (denom, price) = item?;
            Ok(PriceInfo {
                denom,
                price_usd: price,
            })
        })
        .collect()
}

pub fn vault_state(deps: Deps) -> StdResult<VaultState> {
    Ok(VaultState {
        funds: vault_assets(deps)?,
        total_pending_assets: total_pending_assets(deps)?,
        prices: prices(deps)?,
        tvl: vault_value(deps)?,
    })
}
