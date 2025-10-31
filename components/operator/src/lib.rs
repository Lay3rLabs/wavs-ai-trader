wit_bindgen::generate!({
    world: "wavs-world",
    generate_all,
});

mod coingecko;
mod core;
mod skip;

use ai_portfolio_types::TradeStrategy;
use layer_climb::{
    prelude::{AddrKind, Address, ChainConfig, ChainId, CosmosAddr},
    querier::QueryClient,
};
use wstd::runtime::block_on;

use crate::{
    core::generate_payload,
    wavs::{
        operator::input::TriggerData,
        types::{
            core::Timestamp,
            events::{TriggerDataCosmosContractEvent, TriggerDataCron},
        },
    },
};

struct Component;

impl Guest for Component {
    fn run(action: TriggerAction) -> std::result::Result<Option<WasmResponse>, String> {
        let trigger_time = match action.data {
            TriggerData::CosmosContractEvent(TriggerDataCosmosContractEvent { event, .. }) => {
                // Search through attributes to find the one with key "trigger_time"
                let trigger_time_str = event
                    .attributes
                    .iter()
                    .find(|(key, _)| key == "trigger_time")
                    .map(|(_, value)| value)
                    .ok_or_else(|| {
                        format!("Could not find 'trigger_time' attribute in event {event:?}")
                    })?;

                Ok(Timestamp {
                    nanos: trigger_time_str
                        .parse()
                        .map_err(|e| format!("Could not parse trigger_time '{trigger_time_str}' from event {event:?}: {e}"))?,
                })
            }
            TriggerData::Cron(TriggerDataCron { trigger_time }) => Ok(trigger_time),
            _ => Err(format!(
                "Component did not expect trigger action {action:?}"
            )),
        }?;
        let chain = host::config_var("chain").ok_or("Could not get chain config var")?;
        host::log(
            host::LogLevel::Debug,
            &format!("Chain configured: {}", chain),
        );

        let address = CosmosAddr::new_str(
            &host::config_var("address").ok_or("Could not get address config var")?,
            None,
        )
        .map_err(|e| format!("Could not parse address: {e}"))?;
        host::log(
            host::LogLevel::Debug,
            &format!("Vault address configured: {}", address),
        );

        let chain_config = host::get_cosmos_chain_config(&chain)
            .ok_or(format!("Could not get chain config for {chain}"))?;

        let payload = block_on(async move {
            let query_client = QueryClient::new(
                ChainConfig {
                    chain_id: ChainId::new(chain_config.chain_id.clone()),
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
                &host::config_var("trade_strategy").ok_or("Could not get the trade strategy")?,
            )
            .map_err(|e| format!("Error parsing the trade strategy: {e}"))?;

            host::log(host::LogLevel::Info, "Starting payload generation...");

            let result = generate_payload(
                query_client,
                Address::Cosmos(address),
                trade_strategy,
                trigger_time.nanos,
                chain_config.chain_id,
            )
            .await;

            if result.is_ok() {
                host::log(
                    host::LogLevel::Info,
                    "Payload generation completed successfully",
                );
            }

            result.map_err(|e| e.to_string())
        })?;

        let response = WasmResponse {
            payload: payload
                .to_bytes()
                .map_err(|e| format!("Could not encode payload: {e}"))?,
            ordering: None,
        };

        Ok(Some(response))
    }
}

export!(Component);
