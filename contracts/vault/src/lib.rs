#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;
use wavs_types::contracts::cosmwasm::service_handler::{
    ServiceHandlerExecuteMessages, ServiceHandlerQueryMessages,
};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, VaultExecuteMsg, VaultQueryMsg,
};
use crate::state::{DEPOSIT_ID_COUNTER, TOTAL_SHARES, VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS};

mod error;
mod execute;
mod msg;
mod query;
mod state;
mod utils;

#[cfg(test)]
mod tests;

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
    let ownership =
        cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // Set initial total shares to zero
    TOTAL_SHARES.save(deps.storage, &cosmwasm_std::Uint256::zero())?;

    // Set initial vault value to zero
    VAULT_VALUE_DEPOSITED.save(deps.storage, &cosmwasm_std::Decimal256::zero())?;

    // Initialize whitelisted denoms
    for denom in msg.initial_whitelisted_denoms {
        WHITELISTED_DENOMS.save(deps.storage, denom, &())?;
    }

    // Initialize deposit_id counter to 0
    DEPOSIT_ID_COUNTER.save(deps.storage, &0u64)?;

    let service_manager = deps.api.addr_validate(&msg.service_manager)?;
    state::SERVICE_MANAGER.save(deps.storage, &service_manager)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attributes(ownership.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Vault(msg) => match msg {
            VaultExecuteMsg::Deposit {} => execute::deposit(deps, env, info),
            VaultExecuteMsg::Withdraw { shares } => execute::withdraw(deps, env, info, shares),
            VaultExecuteMsg::UpdateWhitelist { to_add, to_remove } => {
                execute::update_whitelist(deps, env, info, to_add, to_remove)
            }
            VaultExecuteMsg::UpdatePrices { prices, strategy } => {
                execute::update_prices(deps, env, info, prices, strategy)
            }
            VaultExecuteMsg::UpdateOwnership(action) => {
                let ownership =
                    cw_ownable::update_ownership(deps, &env.block, &info.sender, action.clone())?;

                Ok(Response::new().add_attributes(ownership.into_attributes()))
            }
        },
        ExecuteMsg::Wavs(msg) => match msg {
            ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
                envelope,
                signature_data,
            } => execute::handle_signed_envelope(deps, env, envelope, signature_data),
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    Err(ContractError::UnknownReplyId { id: msg.id })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Vault(msg) => match msg {
            VaultQueryMsg::GetTotalShares {} => to_json_binary(&query::total_shares(deps)?),
            VaultQueryMsg::GetVaultValue {} => to_json_binary(&query::vault_value(deps)?),
            VaultQueryMsg::GetWhitelistedDenoms {} => {
                to_json_binary(&query::whitelisted_denoms(deps)?)
            }
            VaultQueryMsg::GetDepositRequest { deposit_id } => {
                to_json_binary(&query::deposit_request(deps, deposit_id)?)
            }
            VaultQueryMsg::ListDepositRequests { start_after, limit } => {
                to_json_binary(&query::deposit_requests(deps, start_after, limit)?)
            }
            VaultQueryMsg::GetVaultAssets {} => to_json_binary(&query::vault_assets(deps)?),
            VaultQueryMsg::GetVaultAssetBalance { denom } => {
                to_json_binary(&query::vault_asset_balance(deps, denom)?)
            }
            VaultQueryMsg::GetPrice { denom } => to_json_binary(&query::price(deps, denom)?),
            VaultQueryMsg::Ownership {} => {
                Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?)
            }
        },
        QueryMsg::Wavs(msg) => match msg {
            ServiceHandlerQueryMessages::WavsServiceManager {} => {
                to_json_binary(&state::SERVICE_MANAGER.load(deps.storage)?)
            }
        },
    }
}
