#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{
    DEPOSIT_ID_COUNTER, PRICE_ORACLE, TOTAL_SHARES, VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
};

mod error;
mod execute;
mod msg;
mod query;
mod state;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize ownership using cw_ownable
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // Set initial total shares to zero
    TOTAL_SHARES.save(deps.storage, &cosmwasm_std::Uint128::zero())?;

    // Set initial vault value to zero
    VAULT_VALUE_DEPOSITED.save(deps.storage, &cosmwasm_std::Uint128::zero())?;

    // Set price oracle (validate and convert String to Addr)
    let price_oracle_addr = deps.api.addr_validate(&msg.price_oracle)?;
    PRICE_ORACLE.save(deps.storage, &price_oracle_addr)?;

    // Initialize whitelisted denoms
    for denom in msg.initial_whitelisted_denoms {
        WHITELISTED_DENOMS.save(deps.storage, denom, &true)?;
    }

    // Initialize deposit_id counter to 0
    DEPOSIT_ID_COUNTER.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute::deposit(deps, env, info),
        ExecuteMsg::RecordDeposit {
            deposit_id,
            value_usd,
        } => execute::record_deposit(deps, env, info, deposit_id, value_usd),
        ExecuteMsg::Withdraw { shares } => execute::withdraw(deps, env, info, shares),
        ExecuteMsg::UpdatePriceOracle { price_oracle } => {
            execute::update_price_oracle(deps, env, info, price_oracle)
        }
        ExecuteMsg::AddToWhitelist { tokens } => execute::add_to_whitelist(deps, env, info, tokens),
        ExecuteMsg::RemoveFromWhitelist { tokens } => {
            execute::remove_from_whitelist(deps, env, info, tokens)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTotalShares {} => cosmwasm_std::to_json_binary(&query::total_shares(deps)?),
        QueryMsg::GetPriceOracle {} => cosmwasm_std::to_json_binary(&query::price_oracle(deps)?),
        QueryMsg::GetVaultValue {} => cosmwasm_std::to_json_binary(&query::vault_value(deps)?),
        QueryMsg::GetWhitelistedDenoms {} => {
            cosmwasm_std::to_json_binary(&query::whitelisted_denoms(deps)?)
        }
        QueryMsg::GetDepositRequest { deposit_id } => {
            cosmwasm_std::to_json_binary(&query::deposit_request(deps, deposit_id)?)
        }
        QueryMsg::ListDepositRequests { start_after, limit } => {
            cosmwasm_std::to_json_binary(&query::deposit_requests(deps, start_after, limit)?)
        }
        QueryMsg::GetVaultAssets {} => cosmwasm_std::to_json_binary(&query::vault_assets(deps)?),
        QueryMsg::GetVaultAssetBalance { denom } => {
            cosmwasm_std::to_json_binary(&query::vault_asset_balance(deps, denom)?)
        }
    }
}
