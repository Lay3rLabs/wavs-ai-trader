use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Context, Result};
use cosmwasm_std::{Decimal256, Uint256};
use serde::Deserialize;
use wstd::http::{Client, Method, Request};

const SIMPLE_PRICE_ENDPOINT: &str = "https://pro-api.coingecko.com/api/v3/simple/price";

pub struct CoinGeckoApiClient {
    client: Client,
    api_key: Option<String>,
}

impl CoinGeckoApiClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
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

        let mut uri = format!(
            "{}?ids={}&vs_currencies={}",
            SIMPLE_PRICE_ENDPOINT, ids_param, vs_currency
        );

        if let Some(key) = &self.api_key {
            uri.push_str("&x_cg_pro_api_key=");
            uri.push_str(key);
        }

        let req = Request::builder()
            .method(Method::GET)
            .uri(&uri)
            .body(wstd::io::empty())
            .context("failed to build CoinGecko request")?;

        let response = self
            .client
            .send(req)
            .await
            .context("failed to call CoinGecko API")?;

        let status = response.status();
        let mut body = response.into_body();

        if !status.is_success() {
            let bytes = body
                .bytes()
                .await
                .context("failed to read CoinGecko error response")?;
            let message = String::from_utf8(bytes).unwrap_or_else(|_| "<non-utf8 response>".into());
            anyhow::bail!("CoinGecko API returned {status}: {message}");
        }

        let payload: SimplePriceResponse = body
            .json()
            .await
            .context("failed to decode CoinGecko response")?;

        let mut prices = HashMap::new();
        for (denom, id, decimals) in assets {
            if let Some(vs_map) = payload.0.get(id) {
                if let Some(price) = vs_map.get(vs_currency) {
                    let price_decimal = Decimal256::from_str(&price.to_string()).map_err(|e| {
                        anyhow!("failed to parse CoinGecko price into Decimal256: {e}")
                    })?;
                    let scale = Decimal256::from_ratio(
                        Uint256::one(),
                        Uint256::from(10u128.pow(*decimals as u32)),
                    );
                    let atomic_price = price_decimal.checked_mul(scale).map_err(|e| {
                        anyhow!("overflow scaling CoinGecko price into atomic units: {e}")
                    })?;
                    prices.insert(denom.clone(), atomic_price);
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
