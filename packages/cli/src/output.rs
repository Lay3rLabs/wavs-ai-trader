use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wavs_types::ComponentDigest;

use crate::command::{ComponentKind, ContractKind};

pub struct Output {
    pub directory: PathBuf,
    pub path: PathBuf,
    pub format: OutputFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
#[clap(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
}

impl Output {
    pub async fn write(&self, data: OutputData) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                let json_data = serde_json::to_string_pretty(&data)?;
                tokio::fs::write(&self.path, json_data).await?;
            }
        }
        tracing::info!("Output written to {}", self.path.display());

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged, rename_all = "snake_case")]
pub enum OutputData {
    ContractUpload {
        kind: ContractKind,
        code_id: u64,
        tx_hash: String,
    },
    ContractInstantiate {
        kind: ContractKind,
        address: String,
        tx_hash: String,
    },
    ContractMigrate {
        kind: ContractKind,
        address: String,
        new_code_id: u64,
        tx_hash: String,
    },
    ComponentUpload {
        kind: ComponentKind,

        /// The hash of the file,
        digest: ComponentDigest,

        /// The content identifier (CID) of the uploaded file
        cid: String,

        /// The IPFS URI (e.g., "ipfs://Qm...")
        uri: String,

        /// The gateway URL for accessing the file via HTTP
        gateway_url: String,
    },
    ServiceUpload {
        service: wavs_types::Service,
        digest: wavs_types::ServiceDigest,
        cid: String,
        uri: String,
        gateway_url: String,
    },
}
