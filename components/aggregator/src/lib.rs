wit_bindgen::generate!({
    world: "aggregator-world",
    path: "../../wit/aggregator/wit",
    generate_all,
});

use layer_climb::prelude::CosmosAddr;

use crate::wavs::aggregator::aggregator::{CosmosAddress, CosmosSubmitAction, SubmitAction};

struct Component;
impl Guest for Component {
    fn process_packet(_pkt: Packet) -> Result<Vec<AggregatorAction>, String> {
        let chain = host::config_var("chain").ok_or("Could not get chain config var")?;
        let address = CosmosAddr::new_str(
            &host::config_var("address").ok_or("Could not get address config var")?,
            None,
        )
        .map_err(|e| format!("Could not parse address: {e}"))?;

        let submit_action = SubmitAction::Cosmos(CosmosSubmitAction {
            chain: chain.to_string(),
            address: CosmosAddress {
                bech32_addr: address.to_string(),
                prefix_len: address.prefix().len() as u32,
            },
            gas_price: None,
        });

        Ok(vec![AggregatorAction::Submit(submit_action)])
    }

    fn handle_timer_callback(_packet: Packet) -> Result<Vec<AggregatorAction>, String> {
        Err("Not implemented yet".to_string())
    }

    fn handle_submit_callback(
        _packet: Packet,
        tx_result: Result<AnyTxHash, String>,
    ) -> Result<(), String> {
        match tx_result {
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
    }
}

export!(Component);
