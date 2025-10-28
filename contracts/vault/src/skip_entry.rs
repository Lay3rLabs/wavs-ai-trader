use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, Timestamp, Uint128};

/// Asset representation compatible with the Skip entry-point contract.
#[cw_serde]
pub enum Asset {
    Native(Coin),
}

/// Swap operation describing a single pool hop.
#[cw_serde]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
    pub interface: Option<Binary>,
}

/// Skip swap payload describing how to execute a swap on a given venue.
#[cw_serde]
pub struct SwapRoute {
    pub swap_venue_name: String,
    pub offer_denom: String,
    pub ask_denom: String,
    pub amount_in: Uint128,
    pub estimated_amount_out: Uint128,
    pub minimum_amount_out: Option<Uint128>,
    pub timeout: Timestamp,
    pub operations: Vec<SwapOperation>,
}

/// Swap instructions accepted by the entry-point contract.
#[cw_serde]
pub struct SwapExactAssetIn {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
}

#[cw_serde]
pub struct SwapExactAssetOut {
    pub swap_venue_name: String,
    pub operations: Vec<SwapOperation>,
    pub refund_address: Option<String>,
}

#[cw_serde]
pub enum Swap {
    SwapExactAssetIn(SwapExactAssetIn),
    SwapExactAssetOut(SwapExactAssetOut),
}

#[cw_serde]
pub struct Affiliate {
    pub basis_points_fee: Uint128,
    pub address: String,
}

#[cw_serde]
pub enum Action {
    Transfer { to_address: String },
}

#[cw_serde]
pub enum ExecuteMsg {
    SwapAndAction {
        sent_asset: Option<Asset>,
        user_swap: Swap,
        min_asset: Asset,
        timeout_timestamp: u64,
        post_swap_action: Action,
        affiliates: Vec<Affiliate>,
    },
}
