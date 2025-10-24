use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128,
};

use crate::error::ContractError;
use crate::msg::PriceUpdate;
use crate::state::{
    DepositRequest, DepositState, DEPOSIT_ID_COUNTER, DEPOSIT_REQUESTS, PRICES, PRICE_ORACLE,
    TOTAL_SHARES, USER_SHARES, VAULT_ASSETS, VAULT_VALUE_DEPOSITED, WHITELISTED_DENOMS,
};

pub fn deposit(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Validate payment using cw_utils helpers
    let coin = cw_utils::one_coin(&info)?;

    // Check if token is whitelisted
    WHITELISTED_DENOMS
        .load(deps.storage, coin.denom.clone())
        .map_err(|_| ContractError::TokenNotWhitelisted {
            token: coin.denom.clone(),
        })?;

    // Generate auto-incrementing deposit_id
    let deposit_id =
        DEPOSIT_ID_COUNTER.update::<_, ContractError>(deps.storage, |id| Ok(id + 1))?;

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
    let vault_value = VAULT_VALUE_DEPOSITED.load(deps.storage)?;

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

    let user_value_usd = Decimal::from_ratio(shares, total_shares).checked_mul(vault_value)?;

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
    total_shares = total_shares.checked_sub(shares)?;

    // Update user's shares
    let updated_user_shares = user_shares.checked_sub(shares)?;

    // Remove user from shares map if they have no shares left
    if updated_user_shares.is_zero() {
        USER_SHARES.remove(deps.storage, info.sender.to_string());
    } else {
        USER_SHARES.save(deps.storage, info.sender.to_string(), &updated_user_shares)?;
    }

    TOTAL_SHARES.save(deps.storage, &total_shares)?;

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
                        .checked_sub(proportion)?;
                    Ok(new_balance)
                },
            )?;

            let balance = vec![Coin {
                denom,
                amount: proportion,
            }];
            Ok::<CosmosMsg, ContractError>(CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: balance,
            }))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    // Recalculate vault value from actual post-withdrawal asset balances
    // This ensures the stored USD value matches the real asset value after truncation
    let updated_vault_value = calculate_vault_usd_value(deps.storage)?;
    VAULT_VALUE_DEPOSITED.save(deps.storage, &updated_vault_value)?;

    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_attribute("method", "withdraw")
        .add_attribute("user", info.sender)
        .add_attribute("shares", shares.to_string())
        .add_attribute("value_usd", user_value_usd.to_string())
        .add_attribute("new_vault_value_usd", updated_vault_value.to_string()))
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

pub fn update_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    to_add: Option<Vec<String>>,
    to_remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner using cw_ownable
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut added_count: u32 = 0;
    let mut removed_count: u32 = 0;

    // Add tokens to whitelist
    if let Some(tokens_to_add) = to_add {
        for token in tokens_to_add {
            // Only save if not already present
            if WHITELISTED_DENOMS
                .may_load(deps.storage, token.clone())?
                .is_none()
            {
                WHITELISTED_DENOMS.save(deps.storage, token, &())?;
                added_count = added_count.checked_add(1).unwrap_or(added_count);
                // If overflow, keep original count
            }
        }
    }

    // Remove tokens from whitelist
    if let Some(tokens_to_remove) = to_remove {
        for token in tokens_to_remove {
            // Only remove if present
            if WHITELISTED_DENOMS
                .may_load(deps.storage, token.clone())?
                .is_some()
            {
                WHITELISTED_DENOMS.remove(deps.storage, token);
                removed_count = removed_count.checked_add(1).unwrap_or(removed_count);
                // If overflow, keep original count
            }
        }
    }

    Ok(Response::new()
        .add_attribute("method", "update_whitelist")
        .add_attribute("updated_by", info.sender)
        .add_attribute("tokens_added", added_count.to_string())
        .add_attribute("tokens_removed", removed_count.to_string()))
}

pub fn update_prices(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    prices: Vec<PriceUpdate>,
) -> Result<Response, ContractError> {
    // Check if sender is the price oracle
    let price_oracle = PRICE_ORACLE.load(deps.storage)?;
    if info.sender != price_oracle {
        return Err(ContractError::Unauthorized {});
    }

    let mut events = Vec::new();

    // Update all provided prices
    for price_update in prices {
        // Validate that the denom is whitelisted
        WHITELISTED_DENOMS
            .load(deps.storage, price_update.denom.clone())
            .map_err(|_| ContractError::TokenNotWhitelisted {
                token: price_update.denom.clone(),
            })?;

        // Store the new price
        PRICES.save(
            deps.storage,
            price_update.denom.clone(),
            &price_update.price_usd,
        )?;

        events.push(
            cosmwasm_std::Event::new("price_updated")
                .add_attribute("denom", &price_update.denom)
                .add_attribute("price_usd", price_update.price_usd.to_string()),
        );
    }

    // Calculate new total vault value based on current prices
    let new_vault_value = calculate_vault_usd_value(deps.storage)?;

    // Process all pending deposits
    let (processed_deposits, final_vault_value) =
        process_pending_deposits(deps.storage, new_vault_value)?;

    // Update the stored vault value to include all processed deposits
    VAULT_VALUE_DEPOSITED.save(deps.storage, &final_vault_value)?;

    // Add events for processed deposits
    let processed_count = processed_deposits.len();
    for deposit_info in &processed_deposits {
        events.push(
            cosmwasm_std::Event::new("deposit_processed")
                .add_attribute("deposit_id", deposit_info.deposit_id.to_string())
                .add_attribute("user", &deposit_info.user)
                .add_attribute("value_usd", deposit_info.value_usd.to_string())
                .add_attribute("shares_issued", deposit_info.shares_issued.to_string()),
        );
    }

    Ok(Response::new()
        .add_events(events)
        .add_attribute("method", "update_prices")
        .add_attribute("updated_by", info.sender)
        .add_attribute("final_vault_value_usd", final_vault_value.to_string())
        .add_attribute("processed_deposits", processed_count.to_string()))
}

// Calculate the total USD value of all vault assets based on current prices
fn calculate_vault_usd_value(
    storage: &mut dyn cosmwasm_std::Storage,
) -> Result<Decimal, ContractError> {
    let mut total_value = Decimal::zero();

    // Iterate through all vault assets
    for item in VAULT_ASSETS.range(storage, None, None, cosmwasm_std::Order::Ascending) {
        let (denom, balance) = item.map_err(|e| ContractError::Std(e))?;

        // Get the current price for this denom
        if let Some(price_usd) = PRICES.may_load(storage, denom.clone())? {
            // Calculate USD value: balance * price_usd (convert balance to Decimal first)
            // Assuming 0 decimal places for all tokens
            let balance_decimal = Decimal::from_atomics(balance, 0)?;
            let usd_value = price_usd.checked_mul(balance_decimal)?;
            total_value = total_value.checked_add(usd_value)?;
        }
        // If no price is available, we assume the asset has no USD value
        // This could be enhanced to handle missing prices differently
    }

    Ok(total_value)
}

// Process all pending deposits using batch calculation for fair allocation
// Returns processed deposits and the final vault value including all deposits
fn process_pending_deposits(
    storage: &mut dyn cosmwasm_std::Storage,
    vault_value: Decimal,
) -> Result<(Vec<ProcessedDepositInfo>, Decimal), ContractError> {
    let mut processed_deposits = Vec::new();
    let total_shares = TOTAL_SHARES.load(storage)?;

    // Collect all pending deposits and calculate their values first
    let pending_deposits: Vec<(u64, Decimal)> = DEPOSIT_REQUESTS
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            let (id, deposit) = item.ok()?;
            match deposit.state {
                DepositState::Pending => {
                    // Calculate USD value for this deposit
                    if let Ok(price_usd) = PRICES.load(storage, deposit.coin.denom.clone()) {
                        let amount_decimal = Decimal::from_atomics(deposit.coin.amount, 0).ok()?;
                        let value_usd = price_usd.checked_mul(amount_decimal).ok()?;
                        Some((id, value_usd))
                    } else {
                        None
                    }
                }
                DepositState::Completed { .. } => None,
            }
        })
        .collect();

    if pending_deposits.is_empty() {
        return Ok((processed_deposits, vault_value));
    }

    // Calculate total value of all pending deposits
    let total_deposit_value: Decimal = pending_deposits
        .iter()
        .map(|(_, value)| *value)
        .try_fold(Decimal::zero(), |acc, val| acc.checked_add(val))?;

    // Calculate final vault value after all deposits
    let final_vault_value = vault_value.checked_add(total_deposit_value)?;

    // Calculate all share allocations using the standard formula: new_shares = total_shares * value_usd / current_vault_value
    let mut new_shares_for_deposits = Vec::new();
    for (_, value_usd) in &pending_deposits {
        let new_shares = if total_shares.is_zero() {
            // Share precision - 1e6 shares per USD
            let share_precision = Decimal::from_ratio(1000000u128, 1u128);
            // First deposit: shares = value_usd * PRECISION
            value_usd.checked_mul(share_precision)?.to_uint_ceil()
        } else {
            // Standard formula: new_shares = total_shares * value_usd / current_vault_value
            // This ensures proportional share issuance based on current vault value
            let total_shares_decimal = Decimal::from_atomics(total_shares, 0)?;
            let new_shares_decimal = total_shares_decimal
                .checked_mul(*value_usd)?
                .checked_div(vault_value)?;
            new_shares_decimal.to_uint_ceil()
        };
        new_shares_for_deposits.push(new_shares);
    }

    // Process each deposit with pre-calculated shares
    let mut new_total_shares = total_shares;
    for (i, (deposit_id, value_usd)) in pending_deposits.into_iter().enumerate() {
        let mut deposit_request = DEPOSIT_REQUESTS.load(storage, deposit_id)?;
        let new_shares = new_shares_for_deposits[i];

        // Only process if still pending
        if let DepositState::Pending = deposit_request.state {
            // Update deposit state to completed
            deposit_request.state = DepositState::Completed { value_usd };
            DEPOSIT_REQUESTS.save(storage, deposit_id, &deposit_request)?;

            // Update user's shares
            USER_SHARES.update(
                storage,
                deposit_request.user.to_string(),
                |user_shares| -> Result<_, ContractError> {
                    let current_shares = user_shares.unwrap_or_default();
                    let updated_shares = current_shares.checked_add(new_shares)?;
                    Ok(updated_shares)
                },
            )?;

            // Add the deposited coin to vault assets
            VAULT_ASSETS.update(
                storage,
                deposit_request.coin.denom.clone(),
                |balance| -> Result<_, ContractError> {
                    let current_balance = balance.unwrap_or_default();
                    let updated_balance =
                        current_balance.checked_add(deposit_request.coin.amount)?;
                    Ok(updated_balance)
                },
            )?;

            // Update total shares for next calculation
            new_total_shares = new_total_shares.checked_add(new_shares)?;

            // Record the processed deposit info for events
            processed_deposits.push(ProcessedDepositInfo {
                deposit_id,
                user: deposit_request.user.to_string(),
                value_usd,
                shares_issued: new_shares,
            });
        }
    }

    // Save the final total shares
    TOTAL_SHARES.save(storage, &new_total_shares)?;

    Ok((processed_deposits, final_vault_value))
}

// Helper struct to track processed deposit information for events
struct ProcessedDepositInfo {
    deposit_id: u64,
    user: String,
    value_usd: Decimal,
    shares_issued: Uint128,
}
