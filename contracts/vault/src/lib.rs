#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    Uint256,
};
use cw2::set_contract_version;
use wavs_types::contracts::cosmwasm::service_handler::{
    ServiceHandlerExecuteMessages, ServiceHandlerQueryMessages,
};

use crate::error::ContractError;
use crate::execute::calculate_vault_usd_value;
use crate::state::{
    DEPOSIT_ID_COUNTER, SKIP_ENTRY_POINT, TOTAL_SHARES, TRADE_TRACKER, VAULT_ASSETS,
    VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
};

mod error;
mod execute;
pub mod msg;
mod query;
mod skip_entry;
mod state;

pub use msg::*;
pub use skip_entry::{SwapOperation, SwapRoute};

#[cfg(test)]
mod tests;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_TRACKER_ID: u64 = 1u64;

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
        WHITELISTED_DENOMS.save(deps.storage, denom.clone(), &())?;
    }

    // Initialize deposit_id counter to 0
    DEPOSIT_ID_COUNTER.save(deps.storage, &0u64)?;

    let service_manager = deps.api.addr_validate(&msg.service_manager)?;
    state::SERVICE_MANAGER.save(deps.storage, &service_manager)?;

    let skip_entry_point = deps.api.addr_validate(&msg.skip_entry_point)?;
    SKIP_ENTRY_POINT.save(deps.storage, &skip_entry_point)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("skip_entry_point", skip_entry_point)
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
            VaultExecuteMsg::UpdatePrices {
                prices,
                swap_routes,
            } => execute::update_prices(deps, env, info, prices, swap_routes),
            VaultExecuteMsg::UpdateOwnership(action) => {
                let ownership =
                    cw_ownable::update_ownership(deps, &env.block, &info.sender, action.clone())?;

                Ok(Response::new().add_attributes(ownership.into_attributes()))
            }
            VaultExecuteMsg::ManualTrigger {} => execute::manual_trigger(deps, env, info),
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_TRACKER_ID => {
            let msg_response = msg.result.into_result().map_err(StdError::msg)?;

            if let Some(trade_info) = TRADE_TRACKER.pop_front(deps.storage)? {
                // Extract swap output information from reply attributes
                let mut swap_out_amount: Option<Uint256> = None;
                let mut swap_out_denom: Option<String> = None;

                for event in msg_response.events {
                    for attribute in event.attributes {
                        match attribute.key.as_str() {
                            "post_swap_action_amount_out" => {
                                swap_out_amount = Some(
                                    attribute
                                        .value
                                        .parse::<u128>()
                                        .map_err(|_| {
                                            ContractError::Std(StdError::msg(format!(
                                                "Failed to parse amount: {}",
                                                attribute.value
                                            )))
                                        })?
                                        .into(),
                                );
                            }
                            "post_swap_action_denom_out" => {
                                swap_out_denom = Some(attribute.value);
                            }
                            _ => {}
                        }
                    }
                }

                // Update vault assets using only the swap deltas
                // Incoming asset: add only the amount received from the swap (from reply attributes)
                if let (Some(denom), Some(amount)) = (swap_out_denom.clone(), swap_out_amount) {
                    VAULT_ASSETS.update::<_, ContractError>(
                        deps.storage,
                        denom,
                        |current_balance| -> Result<_, ContractError> {
                            let balance = current_balance.unwrap_or_default();
                            let new_balance = balance.checked_add(amount)?;
                            Ok(new_balance)
                        },
                    )?;
                } else {
                    return Err(ContractError::Std(StdError::msg(
                        "Could not get post swap action attributes",
                    )));
                }

                // Outgoing asset: subtract the amount we sent in the swap
                VAULT_ASSETS.update::<_, ContractError>(
                    deps.storage,
                    trade_info.in_coin.denom.clone(),
                    |current_balance| -> Result<_, ContractError> {
                        let balance = current_balance.unwrap_or_default();
                        let new_balance = balance.checked_sub(trade_info.in_coin.amount)?;
                        Ok(new_balance)
                    },
                )?;

                // Add trade completion event with actual swap amounts
                let mut response = Response::new().add_event(
                    cosmwasm_std::Event::new("trade_completed")
                        .add_attribute("in_denom", &trade_info.in_coin.denom)
                        .add_attribute("in_amount", trade_info.in_coin.amount.to_string())
                        .add_attribute(
                            "out_denom",
                            swap_out_denom.unwrap_or_else(|| trade_info.out_denom.clone()),
                        )
                        .add_attribute(
                            "out_amount",
                            swap_out_amount
                                .map(|a| a.to_string())
                                .unwrap_or_else(|| "unknown".to_string()),
                        ),
                );

                // Once all operations are completed, then calculate vault value again
                if TRADE_TRACKER.is_empty(deps.storage)? {
                    let updated_vault_value = calculate_vault_usd_value(deps.storage)?;
                    VAULT_VALUE_DEPOSITED.save(deps.storage, &updated_vault_value)?;

                    response = response.add_event(
                        cosmwasm_std::Event::new("trade_finalized")
                            .add_attribute("vault_value_usd", updated_vault_value.to_string()),
                    );
                }

                Ok(response)
            } else {
                Ok(Response::new())
            }
        }
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
            VaultQueryMsg::GetTotalPendingAssets {} => {
                to_json_binary(&query::total_pending_assets(deps)?)
            }
            VaultQueryMsg::GetPendingAssetBalance { denom } => {
                to_json_binary(&query::pending_asset_balance(deps, denom)?)
            }
            VaultQueryMsg::GetPrice { denom } => to_json_binary(&query::price(deps, denom)?),
            VaultQueryMsg::Ownership {} => {
                Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?)
            }
            VaultQueryMsg::GetVaultState {} => to_json_binary(&query::vault_state(deps)?),
            VaultQueryMsg::GetPrices {} => to_json_binary(&query::prices(deps)?),
            VaultQueryMsg::GetUserShares { user } => {
                to_json_binary(&query::user_shares(deps, user)?)
            }
        },
        QueryMsg::Wavs(msg) => match msg {
            ServiceHandlerQueryMessages::WavsServiceManager {} => {
                to_json_binary(&state::SERVICE_MANAGER.load(deps.storage)?)
            }
        },
    }
}
