use std::collections::BTreeMap;

use anyhow::{ensure, Result};
use layer_climb::{prelude::Address, querier::QueryClient};
use rust_decimal::Decimal as RustDecimal;
use serde::{Deserialize, Serialize};
use vault::{Payload, QueryMsg, VaultQueryMsg, VaultState};

#[derive(Serialize, Deserialize)]
pub enum TradeStrategy {
    AI, // Placeholder for now
    Fixed(BTreeMap<String, RustDecimal>),
}

impl TradeStrategy {
    pub fn validate(&self) -> Result<()> {
        match self {
            TradeStrategy::AI => {}
            TradeStrategy::Fixed(map) => {
                let total: RustDecimal = map.values().sum();

                ensure!(
                    total == RustDecimal::ONE,
                    "Total fixed allocation must be equal to one"
                )
            }
        };

        Ok(())
    }
}

pub async fn generate_payload(
    query_client: QueryClient,
    addr: Address,
    trade_strategy: TradeStrategy,
    timestamp: u64,
) -> Result<Payload> {
    trade_strategy.validate()?;

    let _vault_state: VaultState = query_client
        .contract_smart(&addr, &QueryMsg::Vault(VaultQueryMsg::GetVaultState {}))
        .await?;

    // TODO

    Ok(Payload {
        timestamp: cosmwasm_std::Timestamp::from_nanos(timestamp),
        prices: vec![],
        swap_operations: None,
    })
}
