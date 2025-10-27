use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};

#[cw_serde]
pub struct SwapOperations {
    pub operations: Vec<SwapOperation>,
    pub minimum_receive: Option<Uint128>,
    pub max_spread: Option<Decimal>,
    pub coin: Coin,
}

#[cw_serde]
pub enum SwapOperation {
    /// Native swap
    NativeSwap {
        /// The name (denomination) of the native asset to swap from
        offer_denom: String,
        /// The name (denomination) of the native asset to swap to
        ask_denom: String,
    },
    /// ASTRO swap
    AstroSwap {
        /// Information about the asset being swapped
        offer_asset_info: AssetInfo,
        /// Information about the asset we swap to
        ask_asset_info: AssetInfo,
    },
}

#[cw_serde]
#[derive(Hash, Eq)]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

#[cw_serde]
pub struct SwapResponseData {
    pub return_amount: Uint128,
}
