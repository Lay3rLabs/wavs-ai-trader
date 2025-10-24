use cosmwasm_std::{Coin, Decimal, Deps, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::state::{
    DepositRequest, DEPOSIT_REQUESTS, PRICES, PRICE_ORACLE, TOTAL_SHARES, VAULT_ASSETS,
    VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
};

pub fn total_shares(deps: Deps) -> StdResult<Uint128> {
    TOTAL_SHARES.load(deps.storage)
}

pub fn price_oracle(deps: Deps) -> StdResult<String> {
    let price_oracle = PRICE_ORACLE.load(deps.storage)?;
    Ok(price_oracle.into_string())
}

pub fn vault_value(deps: Deps) -> StdResult<Decimal> {
    VAULT_VALUE_DEPOSITED.load(deps.storage)
}

pub fn whitelisted_denoms(deps: Deps) -> StdResult<Vec<String>> {
    WHITELISTED_DENOMS
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
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
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit.unwrap_or(30) as usize)
        .map(|item| item.map(|(_, deposit)| deposit))
        .collect()
}

pub fn vault_assets(deps: Deps) -> StdResult<Vec<Coin>> {
    VAULT_ASSETS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

pub fn vault_asset_balance(deps: Deps, denom: String) -> StdResult<Uint128> {
    VAULT_ASSETS
        .load(deps.storage, denom)
        .or(Ok(Uint128::zero()))
}

pub fn price(deps: Deps, denom: String) -> StdResult<Decimal> {
    PRICES.load(deps.storage, denom).or(Ok(Decimal::zero()))
}
