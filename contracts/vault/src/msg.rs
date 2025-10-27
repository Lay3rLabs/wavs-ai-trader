use crate::astroport::SwapOperations;
use bincode::error::{DecodeError, EncodeError};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal256, Timestamp, Uint256};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use serde::{Deserialize, Serialize};
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
        prices: Vec<PriceInfo>,
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
    #[returns(Vec<Coin>)]
    GetPendingAssets {},
    #[returns(Uint256)]
    GetPendingAssetBalance { denom: String },
    #[returns(cosmwasm_std::Decimal256)]
    GetPrice { denom: String },
    #[returns(Vec<PriceInfo>)]
    GetPrices {},
    #[returns(VaultState)]
    GetVaultState {},
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
pub struct MigrateMsg {}

#[cw_serde]
pub struct VaultState {
    pub funds: Vec<Coin>,
    pub pending_assets: Vec<Coin>,
    pub prices: Vec<PriceInfo>,
    pub tvl: Decimal256,
}

#[cw_serde]
pub struct PriceInfo {
    pub denom: String,
    pub price_usd: Decimal256, // Price as USD decimal (e.g., 1234.56)
}

#[cw_serde]
pub struct DepositRequest {
    pub id: u64,
    pub user: Addr,
    pub coins: Vec<Coin>,
    pub state: DepositState,
}

#[cw_serde]
pub enum DepositState {
    Pending,
    Completed { value_usd: Decimal256 },
}

#[derive(Serialize, Deserialize)]
pub struct Payload {
    pub timestamp: Timestamp,
    pub prices: Vec<PriceInfo>,
    pub swap_operations: Option<Vec<SwapOperations>>,
}

impl Payload {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(bincode::serde::decode_from_slice(bytes, bincode::config::standard())?.0)
    }
}
