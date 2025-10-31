use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Context, Result};
use cosmwasm_std::Decimal256;
use rust_decimal::Decimal;
use serde::Deserialize;
use wavs_wasi_utils::http;
use wstd::http::Request;

use crate::host;

const SIMPLE_PRICE_ENDPOINT: &str = "https://api.coingecko.com/api/v3/simple/price";

pub struct CoinGeckoApiClient {
    api_key: Option<String>,
}

impl CoinGeckoApiClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    /// Helper to add a header to a request
    fn add_header_to_request<B>(
        mut request: Request<B>,
        key: &'static str,
        value: String,
    ) -> Result<Request<B>> {
        request.headers_mut().insert(key, value.parse()?);
        Ok(request)
    }

    pub async fn query_prices(
        &self,
        assets: &[(String, String, u8)],
        vs_currency: &str,
    ) -> Result<HashMap<String, Decimal256>> {
        if assets.is_empty() {
            return Ok(HashMap::new());
        }

        let ids_param = assets
            .iter()
            .map(|(_, id, _)| id.as_str())
            .collect::<Vec<&str>>()
            .join(",");

        let uri = format!(
            "{}?ids={}&vs_currencies={}",
            SIMPLE_PRICE_ENDPOINT, ids_param, vs_currency
        );

        // Create GET request with optional API key header
        let request = http::http_request_get(&uri)?;

        let request = if let Some(key) = &self.api_key {
            // Add API key header if present
            Self::add_header_to_request(request, "x-cg-demo-api-key", key.clone())?
        } else {
            request
        };

        host::log(
            host::LogLevel::Info,
            &format!("Making CoinGecko API request to: {}", uri),
        );

        let payload: SimplePriceResponse = http::fetch_json(request)
            .await
            .context("failed to call CoinGecko API")?;

        host::log(
            host::LogLevel::Info,
            "CoinGecko API response received successfully",
        );

        let mut prices = HashMap::new();
        for (denom, id, _) in assets {
            if let Some(vs_map) = payload.0.get(id) {
                if let Some(price) = vs_map.get(vs_currency) {
                    // Use rust_decimal for parsing to maintain precision
                    let rust_decimal = Decimal::from_str(&price.to_string()).map_err(|e| {
                        anyhow!("failed to parse CoinGecko price into rust_decimal: {e}")
                    })?;

                    // Convert to Decimal256 for the payload (raw USD price, no scaling)
                    let price_decimal =
                        Decimal256::from_str(&rust_decimal.to_string()).map_err(|e| {
                            anyhow!("failed to convert rust_decimal to Decimal256: {e}")
                        })?;

                    prices.insert(denom.clone(), price_decimal);
                }
            }
        }

        Ok(prices)
    }
}

// Map built from https://docs.skip.build/go/api-reference/prod/fungible/get-v2fungibleassets
// This could be a cached query, but we hardcode here for simplicity
pub fn get_neutron_asset(denom: &str) -> Option<(String, u8)> {
    let (denom, decimals) = match denom {
        "ibc/0E293A7622DC9A6439DB60E6D234B5AF446962E27CA3AB44D0590603DFF6968E" => ("bitcoin", 8),
        "untrn" => ("neutron-3", 6),
        "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81" => ("usd-coin", 6),
        "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9" => ("cosmos", 6),
        "ibc/2CB87BCE0937B1D1DFCEE79BE4501AAF3C265E923509AEAC410AD85D27F35130" => {
            ("dydx-chain", 18)
        }
        "ibc/4D04085167777659C11784A356D6B0D13D5C7F0CE77F7DB1152FE03A2DE2CBF2" => {
            ("bridged-wrapped-steth-axelar", 18)
        }
        _ => return None,
    };

    Some((denom.to_string(), decimals))
}

#[derive(Debug, Deserialize)]
struct SimplePriceResponse(HashMap<String, HashMap<String, f64>>);
