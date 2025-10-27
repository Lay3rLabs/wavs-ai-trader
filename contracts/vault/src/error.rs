use cosmwasm_std::{CheckedFromRatioError, Decimal256RangeExceeded, OverflowError, StdError};
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    Ownership(#[from] OwnershipError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] Decimal256RangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Token not whitelisted: {token}")]
    TokenNotWhitelisted { token: String },

    #[error("Insufficient shares")]
    InsufficientShares {},

    #[error("Price must be greater than zero for denom: {denom}")]
    ZeroPrice { denom: String },

    #[error("Deposit already completed: {deposit_id}")]
    DepositAlreadyCompleted { deposit_id: u64 },

    #[error("Unknown reply id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Cannot withdraw zero shares")]
    ZeroWithdrawal {},

    #[error("No funds provided")]
    NoFunds {},

    #[error("Invalid percentages: must sum to 100%")]
    InvalidPercentages {},

    #[error("Duplicate denom: {denom}")]
    DuplicateDenom { denom: String },
}
