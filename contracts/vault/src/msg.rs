use crate::state::DepositRequest;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub price_oracle: String,
    pub initial_whitelisted_denoms: Vec<String>,
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Uint128)]
    GetTotalShares {},
    #[returns(String)]
    GetPriceOracle {},
    #[returns(cosmwasm_std::Decimal)]
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
    #[returns(cosmwasm_std::Decimal)]
    GetPrice { denom: String },
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Deposit {},
    Withdraw {
        shares: Uint128,
    },
    UpdatePriceOracle {
        price_oracle: String,
    },
    UpdateWhitelist {
        to_add: Option<Vec<String>>,
        to_remove: Option<Vec<String>>,
    },
    UpdatePrices {
        prices: Vec<PriceUpdate>,
    },
}

#[cw_serde]
pub struct PriceUpdate {
    pub denom: String,
    pub price_usd: cosmwasm_std::Decimal, // Price as USD decimal (e.g., 1234.56)
}

#[cw_serde]
pub struct MigrateMsg {}
