use crate::{astroport::SwapOperations, state::DepositRequest};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint256};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use wavs_types::contracts::cosmwasm::service_handler::{
    ServiceHandlerExecuteMessages, ServiceHandlerQueryMessages,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub service_manager: String,
    pub initial_whitelisted_denoms: Vec<String>,
    pub astroport_router: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum VaultExecuteMsg {
    Deposit {},
    Withdraw {
        shares: Uint256,
    },
    UpdateWhitelist {
        to_add: Option<Vec<String>>,
        to_remove: Option<Vec<String>>,
    },
    UpdatePrices {
        prices: Vec<PriceUpdate>,
        swap_operations: Option<Vec<SwapOperations>>,
    },
}

#[cw_serde]
#[serde(untagged)]
pub enum ExecuteMsg {
    Vault(VaultExecuteMsg),
    Wavs(ServiceHandlerExecuteMessages),
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum VaultQueryMsg {
    #[returns(Uint256)]
    GetTotalShares {},
    #[returns(cosmwasm_std::Decimal256)]
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
    #[returns(Uint256)]
    GetVaultAssetBalance { denom: String },
    #[returns(cosmwasm_std::Decimal256)]
    GetPrice { denom: String },
}

#[cw_serde]
#[derive(QueryResponses)]
#[query_responses(nested)]
#[serde(untagged)]
pub enum QueryMsg {
    Vault(VaultQueryMsg),
    Wavs(ServiceHandlerQueryMessages),
}

#[cw_serde]
pub struct PriceUpdate {
    pub denom: String,
    pub price_usd: cosmwasm_std::Decimal256, // Price as USD decimal (e.g., 1234.56)
}

#[cw_serde]
pub struct MigrateMsg {}
