use anyhow::{anyhow, Context, Result};
use cosmwasm_std::Uint128;
use serde::Serialize;
use wstd::http::{Body, Client, Method, Request};

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
            swap_venues: vec![SwapVenue {
                name: "neutron-astroport".to_string(),
                chain_id,
                logo_uri: None,
            }],
        }
    }

    pub async fn plan_route(
        &self,
        source_asset_denom: &str,
        dest_asset_denom: &str,
        amount_in: Uint128,
    ) -> Result<RoutePlan> {
        let request = RouteRequest {
            source_asset_denom: source_asset_denom.to_string(),
            source_asset_chain_id: self.chain_id.clone(),
            dest_asset_denom: dest_asset_denom.to_string(),
            dest_asset_chain_id: self.chain_id.clone(),
            amount_in: Some(amount_in.to_string()),
            amount_out: None,
            swap_venues: self
                .swap_venues
                .iter()
                .map(|venue| RouteSwapVenue {
                    name: venue.name.clone(),
                    chain_id: venue.chain_id.clone(),
                })
                .collect(),
            allow_multi_tx: false,
        };

        let req = Request::builder()
            .method(Method::POST)
            .uri(ROUTE)
            .body(Body::from_json(&request).context("failed to serialize Skip route request")?)?;

        let client = Client::new();
        let response = client
            .send(req)
            .await
            .context("failed to call Skip route API")?;

        let status = response.status();
        let mut body = response.into_body();

        if !status.is_success() {
            let message = body
                .str_contents()
                .await
                .context("failed to read Skip route error body")
                .unwrap_or("<non-utf8 response>");
            return Err(anyhow!("Skip route API returned {status}: {message}"));
        }

        body.json::<RoutePlan>()
            .await
            .context("failed to decode Skip route response")
    }
}

#[derive(Serialize)]
struct RouteRequest {
    source_asset_denom: String,
    source_asset_chain_id: String,
    dest_asset_denom: String,
    dest_asset_chain_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount_out: Option<String>,
    swap_venues: Vec<RouteSwapVenue>,
    allow_multi_tx: bool,
}

#[derive(Serialize)]
struct RouteSwapVenue {
    name: String,
    chain_id: String,
}
