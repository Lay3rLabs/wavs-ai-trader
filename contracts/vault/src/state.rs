use bincode::error::{DecodeError, EncodeError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, Uint256, Uint64};
use cw_storage_plus::{Deque, Item, Map};
use serde::{Deserialize, Serialize};
use wavs_types::contracts::cosmwasm::service_handler::{WavsEnvelope, WavsSignatureData};

use crate::{astroport::SwapOperations, msg::PriceUpdate};

#[cw_serde]
pub struct DepositRequest {
    pub id: u64,
    pub user: Addr,
    pub coins: Vec<Coin>,
    pub state: DepositState,
}

#[cw_serde]
pub enum DepositState {
    Pending,
    Completed { value_usd: Decimal256 },
}

#[cw_serde]
pub struct TradeInfo {
    pub in_coin: Coin,
    pub out_denom: String,
}

// Vault
pub const VAULT_VALUE_DEPOSITED: Item<Decimal256> = Item::new("vault_value_deposited");
pub const TOTAL_SHARES: Item<Uint256> = Item::new("total_shares");
pub const WHITELISTED_DENOMS: Map<String, ()> = Map::new("whitelisted_denoms");
pub const DEPOSIT_REQUESTS: Map<u64, DepositRequest> = Map::new("deposit_requests");
pub const VAULT_ASSETS: Map<String, Uint256> = Map::new("vault_assets");
pub const DEPOSIT_ID_COUNTER: Item<u64> = Item::new("deposit_id_counter");
pub const USER_SHARES: Map<String, Uint256> = Map::new("user_shares");
pub const PRICES: Map<String, Decimal256> = Map::new("prices"); // denom -> price_usd
pub const ASTROPORT_ROUTER: Item<Addr> = Item::new("astroport_router");
pub const TRADE_TRACKER: Deque<TradeInfo> = Deque::new("trade_tracker");

// WAVS
pub const SERVICE_MANAGER: Item<Addr> = Item::new("service-manager");
pub const TRIGGER_MESSAGE: Map<Uint64, Vec<PriceUpdate>> = Map::new("trigger-message");
pub const SIGNATURE_DATA: Map<Uint64, WavsSignatureData> = Map::new("signature-data");

#[derive(Serialize, Deserialize)]
pub struct MessageWithId {
    pub trigger_id: Uint64,
    pub prices: Vec<PriceUpdate>,
    pub swap_operations: Option<Vec<SwapOperations>>,
}

impl MessageWithId {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(bincode::serde::decode_from_slice(bytes, bincode::config::standard())?.0)
    }
}

pub fn save_envelope(
    storage: &mut dyn cosmwasm_std::Storage,
    envelope: WavsEnvelope,
    signature_data: WavsSignatureData,
) -> cosmwasm_std::StdResult<MessageWithId> {
    let envelope = envelope.decode()?;
    let message_with_id = MessageWithId::from_bytes(&envelope.payload)?;

    TRIGGER_MESSAGE.save(storage, message_with_id.trigger_id, &message_with_id.prices)?;
    SIGNATURE_DATA.save(storage, message_with_id.trigger_id, &signature_data)?;

    Ok(message_with_id)
}
