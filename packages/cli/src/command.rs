use crate::output::{Output, OutputFormat};
use ai_portfolio_utils::path::repo_root;
use clap::{Parser, ValueEnum};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use wavs_types::ChainKey;

#[derive(Clone, Parser)]
#[command(version, about, long_about = None)]
pub enum CliCommand {
    /// Upload a contract to the chain
    UploadContract {
        #[arg(long)]
        kind: ContractKind,

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
}

// common args for several commands
#[derive(Clone, Debug, Parser)]
pub struct CliArgs {
    #[clap(long, default_value = "cosmos:pion-1")]
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
    pub fn output(&self) -> Output {
        let output_path = repo_root()
            .expect("could not determine repo root")
            .join("builds")
            .join("deployments")
            .join(&self.output_filename);

        // Ensure the output directory exists
        std::fs::create_dir_all(output_path.parent().unwrap()).unwrap_or_else(|_| {
            panic!(
                "Failed to create output directory: {}",
                output_path.parent().unwrap().display()
            )
        });

        Output {
            path: output_path,
            format: self.output_format,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    Payments,
}

impl std::fmt::Display for ContractKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ContractKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Payments => "payments",
        }
    }
    pub async fn wasm_bytes(&self) -> Vec<u8> {
        let path = repo_root()
            .unwrap()
            .join("builds")
            .join("contracts")
            .join(format!("tg_contract_{}.wasm", self.as_str()));

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
            Self::Operator => "operator",
            Self::Aggregator => "aggregator",
        }
    }
    pub async fn wasm_bytes(&self) -> Vec<u8> {
        let path = repo_root()
            .unwrap()
            .join("builds")
            .join("components")
            .join(format!("tg_component_{}.wasm", self.as_str()));

        tokio::fs::read(&path)
            .await
            .unwrap_or_else(|_| panic!("Failed to read wasm bytes at: {}", path.to_string_lossy()))
    }
}
