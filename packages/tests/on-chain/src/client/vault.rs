//! Vault contract client for on-chain testing

use ai_portfolio_utils::{
    addr::AnyAddr,
    client::vault::{VaultExecutor, VaultQuerier},
};

use crate::client::AppClient;

#[derive(Clone)]
pub struct VaultClient {
    pub querier: VaultQuerier,
    pub executor: VaultExecutor,
}

impl VaultClient {
    pub async fn new(app_client: AppClient, addr: Option<cosmwasm_std::Addr>) -> Self {
        let addr = addr.unwrap_or_else(|| cosmwasm_std::Addr::unchecked("vault_contract"));
        let addr_any: AnyAddr = addr.clone().into();

        Self {
            querier: VaultQuerier::new(app_client.querier, addr_any.clone()),
            executor: VaultExecutor::new(app_client.executor, addr_any),
        }
    }
}