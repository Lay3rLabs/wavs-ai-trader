use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositRequest {
    pub id: u64,
    pub user: Addr,
    pub coin: Coin,
    pub state: DepositState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum DepositState {
    Pending,
    Completed { value_usd: Decimal },
}

pub const PRICE_ORACLE: Item<Addr> = Item::new("price_oracle");
pub const VAULT_VALUE_DEPOSITED: Item<Decimal> = Item::new("vault_value_deposited");
pub const TOTAL_SHARES: Item<Uint128> = Item::new("total_shares");
pub const WHITELISTED_DENOMS: Map<String, ()> = Map::new("whitelisted_denoms");
pub const DEPOSIT_REQUESTS: Map<u64, DepositRequest> = Map::new("deposit_requests");
pub const VAULT_ASSETS: Map<String, Uint128> = Map::new("vault_assets");
pub const DEPOSIT_ID_COUNTER: Item<u64> = Item::new("deposit_id_counter");
pub const USER_SHARES: Map<String, Uint128> = Map::new("user_shares");
pub const PRICES: Map<String, Decimal> = Map::new("prices"); // denom -> price_usd
