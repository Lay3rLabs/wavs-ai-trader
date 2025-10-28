use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal256, Uint256, Uint64};
use cw_storage_plus::{Deque, Item, Map};
use wavs_types::contracts::cosmwasm::service_handler::{WavsEnvelope, WavsSignatureData};

use crate::{msg::PriceInfo, DepositRequest, Payload};

#[cw_serde]
pub struct TradeInfo {
    pub in_coin: Coin,
    pub out_denom: String,
    pub timeout_timestamp: u64,
}

// Vault
pub const VAULT_VALUE_DEPOSITED: Item<Decimal256> = Item::new("vault_value_deposited");
pub const TOTAL_SHARES: Item<Uint256> = Item::new("total_shares");
pub const WHITELISTED_DENOMS: Map<String, ()> = Map::new("whitelisted_denoms");
pub const DEPOSIT_REQUESTS: Map<u64, DepositRequest> = Map::new("deposit_requests");
pub const VAULT_ASSETS: Map<String, Uint256> = Map::new("vault_assets");
pub const PENDING_ASSETS: Map<String, Uint256> = Map::new("pending_assets");
pub const DEPOSIT_ID_COUNTER: Item<u64> = Item::new("deposit_id_counter");
pub const USER_SHARES: Map<String, Uint256> = Map::new("user_shares");
pub const PRICES: Map<String, Decimal256> = Map::new("prices"); // denom -> price_usd
pub const SKIP_ENTRY_POINT: Item<Addr> = Item::new("skip_entry_point");
pub const TRADE_TRACKER: Deque<TradeInfo> = Deque::new("trade_tracker");

// WAVS
pub const SERVICE_MANAGER: Item<Addr> = Item::new("service-manager");
pub const TRIGGER_MESSAGE: Map<Uint64, Vec<PriceInfo>> = Map::new("trigger-message");
pub const SIGNATURE_DATA: Map<Uint64, WavsSignatureData> = Map::new("signature-data");

pub fn save_envelope(
    storage: &mut dyn cosmwasm_std::Storage,
    envelope: WavsEnvelope,
    signature_data: WavsSignatureData,
) -> cosmwasm_std::StdResult<Payload> {
    let envelope = envelope.decode()?;
    let payload = Payload::from_bytes(&envelope.payload)?;

    TRIGGER_MESSAGE.save(storage, payload.timestamp.nanos().into(), &payload.prices)?;
    SIGNATURE_DATA.save(storage, payload.timestamp.nanos().into(), &signature_data)?;

    Ok(payload)
}
