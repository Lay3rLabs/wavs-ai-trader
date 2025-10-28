//! Vault contract abstraction for different backends (Climb, Climb Pool, MultiTest)
//! Define helper methods here and they'll be available for all backends

use anyhow::Result;
use cosmwasm_std::{Coin, Decimal256, Uint256};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use crate::{
    addr::AnyAddr,
    client::{AnyExecutor, AnyQuerier, AnyTxResponse},
};

use vault::{
    DepositRequest, ExecuteMsg, PriceInfo, QueryMsg, VaultExecuteMsg, VaultQueryMsg, VaultState,
};

#[derive(Clone)]
pub struct VaultQuerier {
    pub inner: AnyQuerier,
    pub addr: AnyAddr,
}

impl VaultQuerier {
    pub fn new(inner: AnyQuerier, addr: AnyAddr) -> Self {
        Self { inner, addr }
    }

    pub async fn query<RESP: DeserializeOwned + Send + Sync + Debug>(
        &self,
        msg: &QueryMsg,
    ) -> Result<RESP> {
        self.inner.contract_query(&self.addr, msg).await
    }

    /// Query total shares in the vault
    pub async fn total_shares(&self) -> Result<Uint256> {
        let resp: Uint256 = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}))
            .await?;
        Ok(resp)
    }

    /// Query total vault value in USD
    pub async fn vault_value(&self) -> Result<Decimal256> {
        let resp: Decimal256 = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}))
            .await?;
        Ok(resp)
    }

    /// Query whitelisted denominations
    pub async fn whitelisted_denoms(&self) -> Result<Vec<String>> {
        let resp: Vec<String> = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetWhitelistedDenoms {}))
            .await?;
        Ok(resp)
    }

    /// Query a specific deposit request by ID
    pub async fn deposit_request(&self, deposit_id: u64) -> Result<DepositRequest> {
        let resp: DepositRequest = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetDepositRequest {
                deposit_id,
            }))
            .await?;
        Ok(resp)
    }

    /// List deposit requests with pagination
    pub async fn list_deposit_requests(
        &self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Vec<DepositRequest>> {
        let resp: Vec<DepositRequest> = self
            .query(&QueryMsg::Vault(VaultQueryMsg::ListDepositRequests {
                start_after,
                limit,
            }))
            .await?;
        Ok(resp)
    }

    /// Query all vault assets
    pub async fn vault_assets(&self) -> Result<Vec<Coin>> {
        let resp: Vec<Coin> = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetVaultAssets {}))
            .await?;
        Ok(resp)
    }

    /// Query balance of a specific vault asset
    pub async fn vault_asset_balance(&self, denom: String) -> Result<Uint256> {
        let resp: Uint256 = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetVaultAssetBalance {
                denom,
            }))
            .await?;
        Ok(resp)
    }

    /// Query all pending assets
    pub async fn pending_assets(&self) -> Result<Vec<Coin>> {
        let resp: Vec<Coin> = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetPendingAssets {}))
            .await?;
        Ok(resp)
    }

    /// Query balance of a specific pending asset
    pub async fn pending_asset_balance(&self, denom: String) -> Result<Uint256> {
        let resp: Uint256 = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetPendingAssetBalance {
                denom,
            }))
            .await?;
        Ok(resp)
    }

    /// Query price of a specific denomination
    pub async fn price(&self, denom: String) -> Result<Decimal256> {
        let resp: Decimal256 = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetPrice { denom }))
            .await?;
        Ok(resp)
    }

    /// Query all prices
    pub async fn prices(&self) -> Result<Vec<PriceInfo>> {
        let resp: Vec<PriceInfo> = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetPrices {}))
            .await?;
        Ok(resp)
    }

    /// Query complete vault state
    pub async fn vault_state(&self) -> Result<VaultState> {
        let resp: VaultState = self
            .query(&QueryMsg::Vault(VaultQueryMsg::GetVaultState {}))
            .await?;
        Ok(resp)
    }
}

#[derive(Clone)]
pub struct VaultExecutor {
    pub inner: AnyExecutor,
    pub addr: AnyAddr,
}

impl VaultExecutor {
    pub fn new(inner: AnyExecutor, addr: AnyAddr) -> Self {
        Self { inner, addr }
    }

    pub async fn exec(&self, msg: &ExecuteMsg, funds: &[Coin]) -> Result<AnyTxResponse> {
        self.inner.contract_exec(&self.addr, msg, funds).await
    }

    /// Execute a deposit to the vault
    pub async fn deposit(&self, funds: &[Coin]) -> Result<AnyTxResponse> {
        self.exec(&ExecuteMsg::Vault(VaultExecuteMsg::Deposit {}), funds)
            .await
    }

    /// Execute a withdrawal from the vault
    pub async fn withdraw(&self, shares: Uint256) -> Result<AnyTxResponse> {
        self.exec(
            &ExecuteMsg::Vault(VaultExecuteMsg::Withdraw { shares }),
            &[],
        )
        .await
    }

    /// Update whitelist (owner only)
    pub async fn update_whitelist(
        &self,
        to_add: Option<Vec<String>>,
        to_remove: Option<Vec<String>>,
    ) -> Result<AnyTxResponse> {
        self.exec(
            &ExecuteMsg::Vault(VaultExecuteMsg::UpdateWhitelist { to_add, to_remove }),
            &[],
        )
        .await
    }

    /// Update prices and optionally execute swaps (internal call)
    pub async fn update_prices(
        &self,
        prices: Vec<PriceInfo>,
        swap_routes: Option<Vec<vault::SwapRoute>>,
    ) -> Result<AnyTxResponse> {
        self.exec(
            &ExecuteMsg::Vault(VaultExecuteMsg::UpdatePrices {
                prices,
                swap_routes,
            }),
            &[],
        )
        .await
    }
}
