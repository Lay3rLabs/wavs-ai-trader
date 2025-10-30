use crate::output::OutputFormat;
use ai_portfolio_types::TradeStrategy;
use ai_portfolio_utils::path::repo_root;
use clap::{Parser, ValueEnum};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wavs_types::ChainKey;

#[derive(Clone, Parser)]
#[command(version, about, long_about = None)]
#[allow(clippy::large_enum_variant)]
pub enum CliCommand {
    /// Upload a contract to the chain
    UploadContract {
        #[arg(long)]
        kind: ContractKind,

        #[clap(flatten)]
        args: CliArgs,
    },
    /// Instantiate the Vault contract
    InstantiateVault {
        /// The code ID of the vault contract
        #[arg(long)]
        code_id: u64,

        /// Initial whitelisted denominations for the vault
        #[arg(long, value_delimiter = ',')]
        initial_whitelisted_denoms: Vec<String>,

        /// The Skip entry point address (if not provided, will use default for the chain)
        #[arg(long)]
        skip_entry_point: Option<String>,

        #[clap(flatten)]
        args: CliArgs,
    },
    /// Migrate the Vault contract
    MigrateVault {
        /// The address of the vault contract to migrate
        #[arg(long)]
        contract_address: String,

        /// The new code ID to migrate to
        #[arg(long)]
        new_code_id: u64,

        #[clap(flatten)]
        args: CliArgs,
    },
    /// Upload a component to IPFS
    UploadComponent {
        #[arg(long)]
        kind: ComponentKind,

        #[arg(long)]
        ipfs_api_url: Url,

        #[arg(long)]
        ipfs_gateway_url: Url,

        #[clap(flatten)]
        args: CliArgs,
    },
    /// Upload a service to IPFS
    UploadService {
        #[arg(long)]
        contract_vault_instantiation_file: PathBuf,

        #[arg(long)]
        middleware_instantiation_file: PathBuf,

        #[arg(long)]
        component_operator_cid_file: PathBuf,

        #[arg(long)]
        component_aggregator_cid_file: PathBuf,

        #[arg(long)]
        cron_schedule: String,

        #[arg(long)]
        trade_strategy: TradeStrategy,

        #[arg(long)]
        aggregator_url: Url,

        #[arg(long)]
        ipfs_api_url: Url,

        #[arg(long)]
        ipfs_gateway_url: Url,

        #[clap(flatten)]
        args: CliArgs,
    },
    FaucetTap {
        /// if not supplied, will be the one in CLI_MNEMONIC
        addr: Option<String>,
        /// if not supplied, will be the default
        amount: Option<u128>,
        /// if not supplied, will be the default
        denom: Option<String>,
        #[clap(flatten)]
        args: CliArgs,
    },
    AssertAccountExists {
        addr: Option<String>,
        #[clap(flatten)]
        args: CliArgs,
    },
    AggregatorRegisterService {
        #[arg(long)]
        service_manager_address: String,

        #[arg(long)]
        aggregator_url: Url,

        #[clap(flatten)]
        args: CliArgs,
    },
    OperatorAddService {
        #[arg(long)]
        service_manager_address: String,

        #[arg(long)]
        wavs_url: Url,

        #[clap(flatten)]
        args: CliArgs,
    },
}

// common args for several commands
#[derive(Clone, Debug, Parser)]
pub struct CliArgs {
    #[clap(long, default_value = "cosmos:neutron-1")]
    pub chain: ChainKey,

    /// Filename for outputting any generated files
    /// which will be written in to `builds/cli/`
    #[clap(long, default_value = "output.json")]
    pub output_filename: String,

    /// Output format for any generated files
    #[clap(long, value_enum, default_value_t = OutputFormat::Json)]
    pub output_format: OutputFormat,
}

impl CliArgs {
    pub fn output(&self) -> crate::output::Output {
        let directory = repo_root()
            .expect("could not determine repo root")
            .join("builds")
            .join("deployments");

        let path = directory.join(&self.output_filename);

        // Ensure the output directory exists
        std::fs::create_dir_all(&directory).unwrap_or_else(|_| {
            panic!("Failed to create output directory: {}", directory.display())
        });

        crate::output::Output {
            directory,
            path,
            format: self.output_format,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    Vault,
}

impl std::fmt::Display for ContractKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ContractKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Vault => "vault",
        }
    }
    pub async fn wasm_bytes(&self) -> Vec<u8> {
        let path = repo_root()
            .unwrap()
            .join("builds")
            .join("contracts")
            .join(format!("{}.wasm", self.as_str()));

        tokio::fs::read(&path)
            .await
            .unwrap_or_else(|_| panic!("Failed to read wasm bytes at: {}", path.to_string_lossy()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    Operator,
    Aggregator,
}

impl std::fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ComponentKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Operator => "ai_portfolio_operator",
            Self::Aggregator => "ai_portfolio_aggregator",
        }
    }
    pub async fn wasm_bytes(&self) -> Vec<u8> {
        let path = repo_root()
            .unwrap()
            .join("builds")
            .join("components")
            .join(format!("{}.wasm", self.as_str()));

        tokio::fs::read(&path)
            .await
            .unwrap_or_else(|_| panic!("Failed to read wasm bytes at: {}", path.to_string_lossy()))
    }
}
