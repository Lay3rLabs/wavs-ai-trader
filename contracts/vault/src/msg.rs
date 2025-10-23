use crate::state::DepositRequest;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub price_oracle: String,
    pub initial_whitelisted_denoms: Vec<String>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Uint128)]
    GetTotalShares {},
    #[returns(String)]
    GetPriceOracle {},
    #[returns(Uint128)]
    GetVaultValue {},
    #[returns(Vec<String>)]
    GetWhitelistedDenoms {},
    #[returns(DepositRequest)]
    GetDepositRequest { deposit_id: u64 },
    #[returns(Vec<DepositRequest>)]
    ListDepositRequests {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<Coin>)]
    GetVaultAssets {},
    #[returns(Uint128)]
    GetVaultAssetBalance { denom: String },
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit {},
    RecordDeposit { deposit_id: u64, value_usd: Uint128 },
    Withdraw { shares: Uint128 },
    UpdatePriceOracle { price_oracle: String },
    AddToWhitelist { tokens: Vec<String> },
    RemoveFromWhitelist { tokens: Vec<String> },
}

#[cw_serde]
pub struct MigrateMsg {}
