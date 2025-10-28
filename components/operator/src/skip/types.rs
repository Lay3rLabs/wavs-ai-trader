use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    pub swap_venues: Vec<SwapVenue>,
    pub txs_required: u32,
    pub usd_amount_in: String,
    pub usd_amount_out: String,

    pub estimated_fees: Vec<serde_json::Value>,
    pub required_chain_addresses: Vec<String>,
    pub estimated_route_duration_seconds: u64,

    pub swap_venue: Option<SwapVenue>,
    pub swap_price_impact_percent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Operation {
    pub swap: Option<Swap>,
    pub transfer: Option<Transfer>,
    pub bank_send: Option<BankSend>,
    pub tx_index: u32,
    pub amount_in: String,
    pub amount_out: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Transfer {
    pub port: String,
    pub channel: String,
    pub from_chain_id: String,
    pub to_chain_id: String,
    pub denom_in: String,
    pub denom_out: String,
    pub bridge_id: String,
    #[serde(default)]
    pub pfm_enabled: Option<bool>,
    #[serde(default)]
    pub supports_memo: Option<bool>,
    #[serde(default)]
    pub smart_relay: Option<bool>,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub dest_denom: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct BankSend {
    pub chain_id: String,
    pub denom: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SwapIn {
    pub swap_venue: SwapVenue,
    pub swap_operations: Vec<SwapOperation>,
    pub swap_amount_in: String,
    pub estimated_amount_out: String,
    pub price_impact_percent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SwapOperation {
    pub pool: String,
    pub denom_in: String,
    pub denom_out: String,
    #[serde(default)]
    pub interface: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SwapVenue {
    pub name: String,
    pub chain_id: String,
    pub logo_uri: Option<String>,
}
