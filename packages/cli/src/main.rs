mod command;
mod context;
mod ipfs;
mod output;

use ai_portfolio_utils::{addresses::skip_swap_entry_point, faucet, tracing::tracing_init};
use vault::InstantiateMsg;

use crate::{command::CliCommand, context::CliContext, ipfs::IpfsFile, output::OutputData};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // Install rustls crypto provider before any TLS operations
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tracing_init();

    let ctx = CliContext::new().await;

    match ctx.command.clone() {
        CliCommand::UploadContract { kind, args } => {
            let client = ctx.signing_client().await.unwrap();

            let (code_id, tx_resp) = client
                .contract_upload_file(kind.wasm_bytes().await, None)
                .await
                .unwrap();

            println!("Uploaded {kind} contract with code ID: {code_id}");

            args.output()
                .write(OutputData::ContractUpload {
                    kind,
                    code_id,
                    tx_hash: tx_resp.txhash,
                })
                .await
                .unwrap();
        }
        CliCommand::FaucetTap {
            addr,
            amount,
            denom,
            args: _,
        } => {
            let client = ctx.query_client().await.unwrap();
            let addr = match addr {
                Some(addr) => ctx.parse_address(&addr).await.unwrap(),
                None => ctx.wallet_addr().await.unwrap(),
            };
            let balance_before = client
                .balance(addr.clone(), None)
                .await
                .unwrap()
                .unwrap_or_default();
            faucet::tap(&addr, amount, denom.as_deref()).await.unwrap();
            let balance_after = client
                .balance(addr.clone(), None)
                .await
                .unwrap()
                .unwrap_or_default();

            println!(
                "Tapped faucet for {addr} - balance before: {balance_before} balance after: {balance_after}"
            );
        }
        CliCommand::InstantiateVault {
            code_id,
            initial_whitelisted_denoms,
            service_manager,
            skip_entry_point,
            args,
        } => {
            let client = ctx.signing_client().await.unwrap();

            // Use provided skip entry point or get default for the chain
            let skip_entry_point = match skip_entry_point {
                Some(addr) => addr,
                None => skip_swap_entry_point(args.chain.id.as_str())
                    .unwrap_or_else(|| {
                        panic!(
                            "No default Skip entry point address configured for chain: {}",
                            args.chain.id
                        )
                    })
                    .to_string(),
            };

            let instantiate_msg = InstantiateMsg {
                service_manager,
                initial_whitelisted_denoms,
                skip_entry_point,
            };

            let (contract_address, tx_resp) = client
                .contract_instantiate(
                    None,
                    code_id,
                    "Vault Contract",
                    &instantiate_msg,
                    vec![],
                    None,
                )
                .await
                .unwrap();

            println!(
                "Instantiated vault contract at address: {} with tx hash: {}",
                contract_address, tx_resp.txhash
            );

            args.output()
                .write(OutputData::ContractInstantiate {
                    kind: crate::command::ContractKind::Vault,
                    address: contract_address.to_string(),
                    tx_hash: tx_resp.txhash,
                })
                .await
                .unwrap();
        }
        CliCommand::UploadComponent {
            kind,
            args,
            ipfs_api_url,
            ipfs_gateway_url,
        } => {
            let bytes = kind.wasm_bytes().await;

            let digest = wavs_types::ComponentDigest::hash(&bytes);

            let resp = IpfsFile::upload(
                bytes,
                &format!("{kind}.wasm"),
                ipfs_api_url.as_ref(),
                ipfs_gateway_url.as_ref(),
                true,
            )
            .await
            .unwrap();

            let IpfsFile {
                cid,
                uri,
                gateway_url,
            } = resp;

            args.output()
                .write(OutputData::ComponentUpload {
                    kind,
                    digest,
                    cid,
                    uri,
                    gateway_url,
                })
                .await
                .unwrap();
        }
    }
}
