#[allow(warnings)]
#[rustfmt::skip]
mod bindings;
mod core;

use crate::{
    bindings::{
        host,
        wavs::{operator::input::TriggerData, types::events::TriggerDataCron},
        WasmResponse,
    },
    core::{generate_payload, TradeStrategy},
};
use bindings::{export, Guest, TriggerAction};
use layer_climb::{
    prelude::{AddrKind, Address, ChainConfig, ChainId, CosmosAddr},
    querier::QueryClient,
};
use wstd::runtime::block_on;

struct Component;

impl Guest for Component {
    fn run(action: TriggerAction) -> std::result::Result<Option<WasmResponse>, String> {
        match action.data {
            TriggerData::Cron(TriggerDataCron { trigger_time }) => {
                let chain = host::config_var("chain").ok_or("Could not get chain config var")?;
                let address = CosmosAddr::new_str(
                    &host::config_var("address").ok_or("Could not get address config var")?,
                    None,
                )
                .map_err(|e| format!("Could not parse address: {e}"))?;
                let chain_config = host::get_cosmos_chain_config(&chain)
                    .ok_or(format!("Could not get chain config for {chain}"))?;

                let payload = block_on(async move {
                    let query_client = QueryClient::new(
                        ChainConfig {
                            chain_id: ChainId::new(chain_config.chain_id),
                            rpc_endpoint: chain_config.rpc_endpoint,
                            grpc_endpoint: chain_config.grpc_endpoint,
                            grpc_web_endpoint: chain_config.grpc_web_endpoint,
                            gas_price: chain_config.gas_price,
                            gas_denom: chain_config.gas_denom,
                            address_kind: AddrKind::Cosmos {
                                prefix: chain_config.bech32_prefix,
                            },
                        },
                        None,
                    )
                    .await
                    .map_err(|e| format!("Could not establish query client for {chain}: {e}"))?;

                    // Get the trade strategy
                    let trade_strategy: TradeStrategy = serde_json::from_str(
                        &host::config_var("trade_strategy")
                            .ok_or("Could not get the trade strategy")?,
                    )
                    .map_err(|e| format!("Error parsing the trade strategy: {e}"))?;

                    generate_payload(
                        query_client,
                        Address::Cosmos(address),
                        trade_strategy,
                        trigger_time.nanos,
                    )
                    .await
                    .map_err(|e| e.to_string())
                })?;

                Ok(Some(WasmResponse {
                    payload: payload
                        .to_bytes()
                        .map_err(|e| format!("Could not encode payload: {e}"))?,
                    ordering: None,
                }))
            }
            _ => Err(format!(
                "Component did not expect trigger action {action:?}"
            )),
        }
    }
}

export!(Component with_types_in bindings);
