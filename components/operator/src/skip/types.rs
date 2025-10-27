use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RoutePlan {
    pub source_asset_denom: String,
    pub source_asset_chain_id: String,
    pub dest_asset_denom: String,
    pub dest_asset_chain_id: String,
    pub amount_in: String,
    pub amount_out: String,

    pub operations: Vec<Operation>,
    pub chain_ids: Vec<String>,
    pub does_swap: bool,
    pub estimated_amount_out: String,

    pub swap_venues: Vec<SwapVenue>, // array
    pub txs_required: u32,
    pub usd_amount_in: String,
    pub usd_amount_out: String,

    pub estimated_fees: Vec<serde_json::Value>,
    pub required_chain_addresses: Vec<String>,
    pub estimated_route_duration_seconds: u64,

    // present in your sample alongside swap_venues
    pub swap_venue: SwapVenue, // single
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Operation {
    pub swap: Swap,
    pub tx_index: u32,
    pub amount_in: String,
    pub amount_out: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Swap {
    pub swap_in: SwapIn,
    pub estimated_affiliate_fee: String,
    pub from_chain_id: String,
    pub chain_id: String,
    pub denom_in: String,
    pub denom_out: String,
    pub swap_venues: Vec<SwapVenue>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapIn {
    pub swap_venue: SwapVenue,
    pub swap_operations: Vec<SwapOperation>,
    pub swap_amount_in: String,
    pub estimated_amount_out: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapVenue {
    pub name: String,
    pub chain_id: String,
}
