mod types;

pub use types::*;

pub const ROUTE: &str = "https://api.skip.build/v2/fungible/route";

pub struct SkipAPIClient {
    chain_id: String, // source = dest
    swap_venues: Vec<SwapVenue>,
}

impl SkipAPIClient {
    pub fn new(chain_id: String) -> Self {
        SkipAPIClient {
            chain_id: chain_id.clone(),
            // hardcoded
            swap_venues: vec![SwapVenue {
                name: "neutron-astroport".to_string(),
                chain_id,
            }],
        }
    }
}
