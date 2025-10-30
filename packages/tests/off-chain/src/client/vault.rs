use ai_portfolio_utils::addr::AnyAddr;
use ai_portfolio_utils::client::vault::{VaultExecutor, VaultQuerier};
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{ContractWrapper, Executor};

use crate::client::AppClient;

#[derive(Clone)]
pub struct VaultClient {
    pub querier: VaultQuerier,
    pub executor: VaultExecutor,
    pub address: Addr,
}

impl VaultClient {
    pub fn new(app_client: AppClient) -> Self {
        let admin = app_client.admin();
        Self::new_with_admin(app_client, admin)
    }

    pub fn new_with_admin(app_client: AppClient, admin: Addr) -> Self {
        let contract = ContractWrapper::new(vault::execute, vault::instantiate, vault::query);
        let code_id = app_client.with_app_mut(|app| app.store_code(Box::new(contract)));

        let msg = vault::msg::InstantiateMsg {
            service_manager: admin.to_string(),
            initial_whitelisted_denoms: vec![
                "uatom".to_string(),
                "uosmo".to_string(),
                "ujuno".to_string(),
            ],
            skip_entry_point: admin.to_string(),
        };

        let address = app_client.with_app_mut(|app| {
            app.instantiate_contract(
                code_id,
                admin.clone(),
                &msg,
                &[],
                "ai-portfolio-vault",
                None,
            )
            .unwrap()
        });

        let querier = VaultQuerier::new(app_client.querier.clone(), address.clone().into());
        let executor = VaultExecutor::new(app_client.executor.clone(), address.clone().into());

        Self {
            querier,
            executor,
            address,
        }
    }

    /// Execute a deposit to the vault
    pub async fn deposit(&self, signer: &AnyAddr, funds: &[Coin]) -> anyhow::Result<()> {
        self.executor.deposit(signer, funds).await?;
        Ok(())
    }

    /// Execute a withdrawal from the vault
    pub async fn withdraw(
        &self,
        signer: &AnyAddr,
        shares: cosmwasm_std::Uint256,
    ) -> anyhow::Result<()> {
        self.executor.withdraw(signer, shares).await?;
        Ok(())
    }

    /// Update prices (contract self-execution)
    pub async fn update_prices(&self, prices: Vec<vault::msg::PriceInfo>) -> anyhow::Result<()> {
        // The update_prices method requires the contract to be the sender
        // This is a contract self-execution pattern
        self.executor
            .update_prices_direct(&self.address, prices, None)
            .await?;
        Ok(())
    }

    /// Update whitelist (admin only) - uses the contract's owner address
    pub async fn update_whitelist(
        &self,
        to_add: Option<Vec<String>>,
        to_remove: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        // Get the actual contract owner from ownership query
        let ownership = self.querier.ownership().await?;
        let owner_addr = ownership.owner.unwrap();

        // Use the actual owner address for the operation
        self.executor
            .update_whitelist_direct(&owner_addr, to_add, to_remove)
            .await?;
        Ok(())
    }
}
