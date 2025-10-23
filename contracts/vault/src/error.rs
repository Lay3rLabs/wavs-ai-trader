use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Ownership error: {0}")]
    Ownership(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Token not whitelisted: {token}")]
    TokenNotWhitelisted { token: String },

    #[error("Insufficient shares")]
    InsufficientShares {},

    #[error("Deposit already completed: {deposit_id}")]
    DepositAlreadyCompleted { deposit_id: u64 },

    #[error("Unknown reply id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Cannot withdraw zero shares")]
    ZeroWithdrawal {},
}
