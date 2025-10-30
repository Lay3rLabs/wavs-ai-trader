use ai_portfolio_utils::{config::load_chain_configs_from_wavs, path::repo_root};
use anyhow::{Context, Result};
use clap::Parser;
use layer_climb::prelude::*;

use crate::command::{CliArgs, CliCommand};

pub struct CliContext {
    pub command: CliCommand,
}

impl CliContext {
    pub async fn new() -> Self {
        if dotenvy::dotenv().is_err() {
            tracing::debug!("Failed to load .env file");
        }

        let command = CliCommand::parse();

        Self { command }
    }

    pub fn args(&self) -> &CliArgs {
        match &self.command {
            CliCommand::UploadContract { args, .. } => args,
            CliCommand::FaucetTap { args, .. } => args,
            CliCommand::UploadComponent { args, .. } => args,
            CliCommand::InstantiateVault { args, .. } => args,
            CliCommand::UploadService { args, .. } => args,
            CliCommand::AssertAccountExists { args, .. } => args,
            CliCommand::AggregatorRegisterService { args, .. } => args,
            CliCommand::OperatorAddService { args, .. } => args,
            CliCommand::MigrateVault { args, .. } => args,
        }
    }

    pub async fn chain_config(&self) -> Result<ChainConfig> {
        let chain_configs = load_chain_configs_from_wavs(Some(
            repo_root().unwrap().join("backend").join("wavs-home"),
        ))
        .await
        .expect("Failed to load chain configurations");

        let chain_config = chain_configs
            .get_chain(&self.args().chain)
            .clone()
            .context(format!("Chain config not found for {}", self.args().chain))?
            .to_cosmos_config()?;

        Ok(chain_config.into())
    }

    pub fn client_mnemonic(&self) -> Result<String> {
        std::env::var("CLI_MNEMONIC")
            .and_then(|m| {
                if m.is_empty() {
                    Err(std::env::VarError::NotPresent)
                } else {
                    Ok(m)
                }
            })
            .context("Mnemonic not found at CLI_MNEMONIC".to_string())
    }

    pub async fn query_client(&self) -> Result<QueryClient> {
        QueryClient::new(self.chain_config().await?, None).await
    }

    pub async fn signing_client(&self) -> Result<SigningClient> {
        let query_client = self.query_client().await?;

        let signer = KeySigner::new_mnemonic_str(&self.client_mnemonic()?, None)?;
        let address = self
            .chain_config()
            .await?
            .address_from_pub_key(&signer.public_key().await?)?;

        let balance = query_client
            .balance(address.clone(), None)
            .await?
            .unwrap_or_default();
        if balance == 0 {
            tracing::warn!("Balance is ZERO, maybe tap the faucet!");
        }
        let signing_client = SigningClient::new(self.chain_config().await?, signer, None).await?;

        Ok(signing_client)
    }

    pub async fn parse_address(&self, addr: &str) -> Result<Address> {
        self.chain_config().await?.parse_address(addr)
    }

    pub async fn wallet_addr(&self) -> Result<Address> {
        let mnemonic = self.client_mnemonic()?;
        let signer = KeySigner::new_mnemonic_str(&mnemonic, None)?;
        let address = self
            .chain_config()
            .await?
            .address_from_pub_key(&signer.public_key().await?)?;
        Ok(address)
    }
}
