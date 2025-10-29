use std::collections::BTreeMap;

use anyhow::{ensure, Result};
use cosmwasm_std::Decimal256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum TradeStrategy {
    AI, // Placeholder for now
    Fixed(BTreeMap<String, Decimal256>),
}

impl TradeStrategy {
    pub fn validate(&self) -> Result<()> {
        match self {
            TradeStrategy::AI => {}
            TradeStrategy::Fixed(map) => {
                let mut total = Decimal256::zero();
                for allocation in map.values() {
                    total = total.checked_add(*allocation)?;
                }

                ensure!(
                    total == Decimal256::one(),
                    "Total fixed allocation must be equal to one"
                )
            }
        };

        Ok(())
    }
}
