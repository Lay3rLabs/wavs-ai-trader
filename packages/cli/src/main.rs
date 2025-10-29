mod command;
mod context;
mod ipfs;
mod output;

use ai_portfolio_utils::{addresses::skip_swap_entry_point, faucet, tracing::tracing_init};
use vault::InstantiateMsg;

use crate::{command::CliCommand, context::CliContext, ipfs::IpfsFile, output::OutputData};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Install rustls crypto provider before any TLS operations
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tracing_init();

    let ctx = CliContext::new().await;

    match ctx.command.clone() {
        CliCommand::UploadContract { kind, args } => {
            let client = ctx.signing_client().await?;

            let (code_id, tx_resp) = client
                .contract_upload_file(kind.wasm_bytes().await, None)
                .await?;

            println!("Uploaded {kind} contract with code ID: {code_id}");

            args.output()
                .write(OutputData::ContractUpload {
                    kind,
                    code_id,
                    tx_hash: tx_resp.txhash,
                })
                .await?;
            Ok(())
        }
        CliCommand::FaucetTap {
            addr,
            amount,
            denom,
            args: _,
        } => {
            let client = ctx.query_client().await?;
            let addr = match addr {
                Some(addr) => ctx.parse_address(&addr).await?,
                None => ctx.wallet_addr().await?,
            };
            let balance_before = client
                .balance(addr.clone(), None)
                .await?
                .unwrap_or_default();
            faucet::tap(&addr, amount, denom.as_deref()).await?;
            let balance_after = client
                .balance(addr.clone(), None)
                .await?
                .unwrap_or_default();

            println!(
                "Tapped faucet for {addr} - balance before: {balance_before} balance after: {balance_after}"
            );
            Ok(())
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

            let (contract_addr, tx_resp) = client
                .contract_instantiate(
                    None,
                    code_id,
                    "AI Portfolio Vault",
                    &instantiate_msg,
                    vec![],
                    None,
                )
                .await
                .unwrap();

            println!(
                "Instantiated vault contract at address: {} with tx hash: {}",
                contract_addr, tx_resp.txhash
            );

            args.output()
                .write(OutputData::ContractInstantiate {
                    kind: crate::command::ContractKind::Vault,
                    address: contract_addr.to_string(),
                    tx_hash: tx_resp.txhash,
                })
                .await?;
            Ok(())
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
            .await?;

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
                .await?;
            Ok(())
        }
        CliCommand::AssertAccountExists { addr, args: _ } => {
            let client = ctx.query_client().await?;
            let addr = match addr {
                Some(addr) => ctx.parse_address(&addr).await?,
                None => ctx.wallet_addr().await?,
            };
            let balance = client
                .balance(addr.clone(), None)
                .await?
                .unwrap_or_default();

            if balance == 0 {
                return Err(anyhow::anyhow!(
                    "{} has zero balance. Please fund the wallet before proceeding.",
                    addr
                ));
            }

            println!("Account {} has balance: {}", addr, balance);
            Ok(())
        }
        CliCommand::UploadService {
            contract_payments_instantiation_file,
            middleware_instantiation_file: _,
            component_operator_cid_file,
            component_aggregator_cid_file,
            cron_schedule,
            aggregator_url,
            ipfs_api_url,
            ipfs_gateway_url,
            args: _,
        } => {
            println!("Uploading service definition to IPFS...");

            // Read component CID files
            let operator_cid = std::fs::read_to_string(&component_operator_cid_file)
                .map_err(|e| anyhow::anyhow!("Failed to read operator CID file: {}", e))?;
            let aggregator_cid = std::fs::read_to_string(&component_aggregator_cid_file)
                .map_err(|e| anyhow::anyhow!("Failed to read aggregator CID file: {}", e))?;

            // Read contract instantiation files
            let payments_instantiation: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&contract_payments_instantiation_file)
                    .map_err(|e| anyhow::anyhow!("Failed to read payments instantiation file: {}", e))?
            )?;

            // TODO: Implement middleware instantiation file reading
            // let middleware_instantiation: serde_json::Value = serde_json::from_str(
            //     &std::fs::read_to_string(&middleware_instantiation_file)?
            // )?;

            // Create service definition
            let service_definition = serde_json::json!({
                "contract_payments": payments_instantiation,
                // "middleware": middleware_instantiation, // TODO: Add when middleware structure is known
                "component_operator_cid": operator_cid.trim(),
                "component_aggregator_cid": aggregator_cid.trim(),
                "cron_schedule": cron_schedule,
                "aggregator_url": aggregator_url.to_string(),
                "created_at": chrono::Utc::now().to_rfc3339()
            });

            // Upload service definition to IPFS
            let service_json = serde_json::to_string_pretty(&service_definition)?;
            let ipfs_file = IpfsFile::upload(
                service_json.as_bytes().to_vec(),
                "service_definition.json",
                ipfs_api_url.as_ref(),
                ipfs_gateway_url.as_ref(),
                true,
            ).await.map_err(|e| anyhow::anyhow!("Failed to upload service to IPFS: {}", e))?;

            println!("Service uploaded successfully!");
            println!("Service CID: {}", ipfs_file.cid);
            println!("Service URI: {}", ipfs_file.uri);
            println!("Gateway URL: {}", ipfs_file.gateway_url);

            // TODO: Implement proper OutputData variant for service uploads
            // args.output().write(OutputData::ServiceUpload { ... }).await?;
            Ok(())
        }
        CliCommand::AggregatorRegisterService {
            service_manager_address,
            aggregator_url,
            args,
        } => {
            println!("Registering service manager {} with aggregator at {}", service_manager_address, aggregator_url);

            // Validate the service manager address
            let validated_address = ctx.parse_address(&service_manager_address).await?;

            // Prepare registration request
            let registration_payload = serde_json::json!({
                "service_manager_address": validated_address.to_string(),
                "chain": args.chain.id.as_str(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            // Send registration request to aggregator
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/api/v1/services/register", aggregator_url))
                .header("Content-Type", "application/json")
                .json(&registration_payload)
                .send()
                .await?;

            if response.status().is_success() {
                let response_json: serde_json::Value = response.json().await?;
                println!("Service registered successfully!");
                println!("Registration response: {}", serde_json::to_string_pretty(&response_json)?);
            } else {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!("Registration failed: {}", error_text));
            }
            Ok(())
        }
        CliCommand::OperatorAddService {
            service_manager_address,
            wavs_url,
            args,
        } => {
            println!("Adding service manager {} to WAVS operator at {}", service_manager_address, wavs_url);

            // Validate the service manager address
            let validated_address = ctx.parse_address(&service_manager_address).await?;

            // Prepare service addition request for WAVS operator
            let service_config = serde_json::json!({
                "service_manager_address": validated_address.to_string(),
                "chain": args.chain.id.as_str(),
                "enabled": true,
                "config": {
                    "max_gas_limit": 500000,
                    "fee_granter": validated_address.to_string()
                }
            });

            // Send service addition request to WAVS operator
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/api/v1/services", wavs_url))
                .header("Content-Type", "application/json")
                .json(&service_config)
                .send()
                .await?;

            if response.status().is_success() {
                let response_json: serde_json::Value = response.json().await?;
                println!("Service added to operator successfully!");
                println!("Service configuration: {}", serde_json::to_string_pretty(&response_json)?);
            } else {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!("Service addition failed: {}", error_text));
            }
            Ok(())
        }
    }
}
