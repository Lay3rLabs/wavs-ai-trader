//! Code IDs for contract deployment

use std::collections::HashMap;
use tokio::sync::OnceCell;

static CODE_IDS: OnceCell<HashMap<String, u64>> = OnceCell::const_new();

pub struct CodeIds;

impl CodeIds {
    pub async fn get() -> &'static HashMap<String, u64> {
        CODE_IDS.get_or_init(Self::initialize).await
    }

    async fn initialize() -> HashMap<String, u64> {
        let mut ids = HashMap::new();
        // In a real implementation, these would be actual uploaded code IDs
        ids.insert("vault".to_string(), 1);
        ids
    }

    pub async fn vault_code_id() -> u64 {
        CodeIds::get().await.get("vault").copied().unwrap_or(1)
    }
}