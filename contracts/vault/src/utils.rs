use cosmwasm_std::{ensure_eq, Decimal256};

use crate::{error::ContractError, msg::DenomAllocation, state::WHITELISTED_DENOMS};

// Validate strategy percentages and denoms
pub fn validate_strategy(
    storage: &dyn cosmwasm_std::Storage,
    strategy: &[DenomAllocation],
) -> Result<(), ContractError> {
    // Check for duplicate denoms
    let mut seen_denoms = std::collections::HashSet::new();
    for allocation in strategy {
        if !seen_denoms.insert(allocation.denom.clone()) {
            return Err(ContractError::DuplicateDenom {
                denom: allocation.denom.clone(),
            });
        }

        // Check if denom is whitelisted
        WHITELISTED_DENOMS
            .load(storage, allocation.denom.clone())
            .map_err(|_| ContractError::TokenNotWhitelisted {
                token: allocation.denom.clone(),
            })?;
    }

    // Check if percentages sum to 100%
    let total_percentage: Decimal256 = strategy
        .iter()
        .map(|allocation| allocation.percentage)
        .try_fold(Decimal256::zero(), |acc, val| acc.checked_add(val))?;

    ensure_eq!(
        total_percentage,
        Decimal256::one(),
        ContractError::InvalidPercentages {}
    );

    Ok(())
}
