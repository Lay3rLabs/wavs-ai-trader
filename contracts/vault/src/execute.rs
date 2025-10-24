use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use cw_utils::NativeBalance;

use crate::error::ContractError;
use crate::state::{
    DepositRequest, DepositState, DEPOSIT_ID_COUNTER, DEPOSIT_REQUESTS, PRICE_ORACLE, TOTAL_SHARES,
    USER_SHARES, VAULT_ASSETS, VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
};

pub fn deposit(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Validate payment using cw_utils helpers
    let coin = cw_utils::one_coin(&info)?;

    // Check if token is whitelisted
    let is_whitelisted = WHITELISTED_DENOMS
        .may_load(deps.storage, coin.denom.clone())?
        .unwrap_or(false);
    if !is_whitelisted {
        return Err(ContractError::TokenNotWhitelisted {
            token: coin.denom.clone(),
        });
    }

    // Generate auto-incrementing deposit_id
    let deposit_id = DEPOSIT_ID_COUNTER.may_load(deps.storage)?.unwrap_or(0) + 1;
    DEPOSIT_ID_COUNTER.save(deps.storage, &deposit_id)?;

    let deposit_request = DepositRequest {
        id: deposit_id,
        user: info.sender.clone(),
        coin: coin.clone(),
        state: DepositState::Pending,
    };

    DEPOSIT_REQUESTS.save(deps.storage, deposit_id, &deposit_request)?;

    Ok(Response::new().add_event(
        cosmwasm_std::Event::new("deposit")
            .add_attribute("deposit_id", deposit_id.to_string())
            .add_attribute("token", &coin.denom)
            .add_attribute("amount", coin.amount.to_string()),
    ))
}

pub fn record_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    deposit_id: u64,
    value_usd: Uint128,
) -> Result<Response, ContractError> {
    let price_oracle = PRICE_ORACLE.load(deps.storage)?;
    if info.sender != price_oracle {
        return Err(ContractError::Unauthorized {});
    }

    let mut deposit_request = DEPOSIT_REQUESTS.load(deps.storage, deposit_id)?;

    // Check if already completed
    match deposit_request.state {
        DepositState::Completed { .. } => {
            return Err(ContractError::DepositAlreadyCompleted { deposit_id });
        }
        DepositState::Pending => {
            // OK to proceed
        }
    }

    // Update to completed state
    deposit_request.state = DepositState::Completed { value_usd };
    DEPOSIT_REQUESTS.save(deps.storage, deposit_id, &deposit_request)?;

    // Add the deposited coin to vault assets
    VAULT_ASSETS.update(
        deps.storage,
        deposit_request.coin.denom.clone(),
        |balance| -> Result<_, ContractError> {
            Ok(balance.unwrap_or_default() + deposit_request.coin.amount)
        },
    )?;

    let mut total_shares = TOTAL_SHARES.load(deps.storage)?;
    let mut vault_value = VAULT_VALUE_DEPOSITED.load(deps.storage)?;

    let total_value_before = vault_value;
    let total_value_after = total_value_before + value_usd;

    // Calculate new shares based on the value being added
    let new_shares = if total_shares.is_zero() {
        // First depositor gets baseline shares
        Uint128::from(1000000u128)
    } else {
        // New shares proportional to their contribution
        total_shares.multiply_ratio(value_usd, total_value_before)
    };

    // Update total shares and vault value
    total_shares += new_shares;
    vault_value = total_value_after;

    // Track user's shares
    USER_SHARES.update(
        deps.storage,
        deposit_request.user.to_string(),
        |user_shares| -> Result<_, ContractError> {
            Ok(user_shares.unwrap_or_default() + new_shares)
        },
    )?;

    TOTAL_SHARES.save(deps.storage, &total_shares)?;
    VAULT_VALUE_DEPOSITED.save(deps.storage, &vault_value)?;

    Ok(Response::new()
        .add_attribute("method", "record_deposit")
        .add_attribute("deposit_id", deposit_id.to_string())
        .add_attribute("user", deposit_request.user)
        .add_attribute("value_usd", value_usd.to_string())
        .add_attribute("shares_issued", new_shares.to_string())
        .add_attribute("total_shares", total_shares.to_string()))
}

pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    shares: Uint128,
) -> Result<Response, ContractError> {
    if shares.is_zero() {
        return Err(ContractError::ZeroWithdrawal {});
    }

    let mut total_shares = TOTAL_SHARES.load(deps.storage)?;
    let mut vault_value = VAULT_VALUE_DEPOSITED.load(deps.storage)?;

    // Check if user has sufficient shares
    let user_shares = USER_SHARES
        .may_load(deps.storage, info.sender.to_string())?
        .unwrap_or_default();
    if user_shares < shares {
        return Err(ContractError::InsufficientShares {});
    }

    if total_shares < shares {
        return Err(ContractError::InsufficientShares {});
    }

    let total_value = vault_value;
    let user_value_usd = shares.multiply_ratio(total_value, total_shares);

    // Store the old total shares for calculation (before subtraction)
    let old_total_shares = total_shares;

    // First, collect all vault assets and calculate proportions
    let assets_to_withdraw: Vec<(String, Uint128, Uint128)> = VAULT_ASSETS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (denom, balance) = item.map_err(|e| ContractError::Std(e))?;
            let proportion = shares.multiply_ratio(balance, old_total_shares);
            Ok::<(String, Uint128, Uint128), ContractError>((denom, balance, proportion))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    // Update the state and vault assets
    total_shares -= shares;
    vault_value = vault_value
        .checked_sub(user_value_usd)
        .map_err(|_| ContractError::InsufficientShares {})?;

    // Update user's shares
    let updated_user_shares = user_shares
        .checked_sub(shares)
        .map_err(|_| ContractError::InsufficientShares {})?;

    // Remove user from shares map if they have no shares left
    if updated_user_shares.is_zero() {
        USER_SHARES.remove(deps.storage, info.sender.to_string());
    } else {
        USER_SHARES.save(deps.storage, info.sender.to_string(), &updated_user_shares)?;
    }

    TOTAL_SHARES.save(deps.storage, &total_shares)?;
    VAULT_VALUE_DEPOSITED.save(deps.storage, &vault_value)?;

    // Create transfer messages and update vault asset balances
    let transfer_msgs = assets_to_withdraw
        .into_iter()
        .map(|(denom, _balance, proportion)| {
            // Update the vault asset balance
            VAULT_ASSETS.update(
                deps.storage,
                denom.clone(),
                |current_balance| -> Result<_, ContractError> {
                    let new_balance = current_balance
                        .unwrap_or_default()
                        .checked_sub(proportion)
                        .map_err(|_| ContractError::InsufficientShares {})?;
                    Ok(new_balance)
                },
            )?;

            let balance = NativeBalance(vec![Coin {
                denom,
                amount: proportion,
            }]);
            Ok::<CosmosMsg, ContractError>(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: balance.into_vec(),
            }))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_attribute("method", "withdraw")
        .add_attribute("user", info.sender)
        .add_attribute("shares", shares.to_string())
        .add_attribute("value_usd", user_value_usd.to_string()))
}

pub fn update_price_oracle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    price_oracle: String,
) -> Result<Response, ContractError> {
    // Check if sender is the owner using cw_ownable
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Validate and convert String to Addr
    let price_oracle_addr = deps.api.addr_validate(&price_oracle)?;

    PRICE_ORACLE.save(deps.storage, &price_oracle_addr)?;

    Ok(Response::new()
        .add_attribute("method", "update_price_oracle")
        .add_attribute("updated_by", info.sender)
        .add_attribute("new_oracle", price_oracle))
}

pub fn add_to_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tokens: Vec<String>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner using cw_ownable
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for token in tokens {
        WHITELISTED_DENOMS.save(deps.storage, token, &true)?;
    }

    Ok(Response::new()
        .add_attribute("method", "add_to_whitelist")
        .add_attribute("updated_by", info.sender))
}

pub fn remove_from_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tokens: Vec<String>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner using cw_ownable
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for token in tokens {
        WHITELISTED_DENOMS.remove(deps.storage, token);
    }

    Ok(Response::new()
        .add_attribute("method", "remove_from_whitelist")
        .add_attribute("updated_by", info.sender))
}
