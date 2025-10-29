mod command;
mod context;
mod ipfs;
mod output;

use ai_portfolio_types::TradeStrategy;
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
            skip_entry_point,
            args,
        } => {
            let client = ctx.signing_client().await?;

            // Read middleware instantiation file to get service manager address
            let middleware_instantiation_file = args.output().directory.join("middleware.json");
            let middleware_content = tokio::fs::read_to_string(&middleware_instantiation_file)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Failed to read middleware instantiation file: {}", e)
                })?;

            let middleware: serde_json::Value = serde_json::from_str(&middleware_content)?;
            let service_manager = middleware["service_manager_address"]
                .as_str()
                .ok_or_else(|| {
                    anyhow::anyhow!("service_manager_address not found in middleware file")
                })?
                .to_string();

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
                .await?;

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
            contract_vault_instantiation_file,
            middleware_instantiation_file,
            component_operator_cid_file,
            component_aggregator_cid_file,
            cron_schedule,
            aggregator_url,
            ipfs_api_url,
            ipfs_gateway_url,
            args,
        } => {
            let output_directory = args.output().directory;

            let contract_vault_instantiation_file =
                output_directory.join(contract_vault_instantiation_file);
            let component_operator_cid_file = output_directory.join(component_operator_cid_file);
            let component_aggregator_cid_file =
                output_directory.join(component_aggregator_cid_file);
            let middleware_instantiation_file =
                output_directory.join(middleware_instantiation_file);

            async fn read_and_decode<T: serde::de::DeserializeOwned>(
                path: std::path::PathBuf,
            ) -> anyhow::Result<T> {
                match tokio::fs::read_to_string(&path).await {
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to read file {}: {}",
                        path.display(),
                        e
                    )),
                    Ok(content) => match serde_json::from_str(&content) {
                        Err(e) => Err(anyhow::anyhow!(
                            "Failed to decode JSON from file {}: {}",
                            path.display(),
                            e
                        )),
                        Ok(data) => Ok(data),
                    },
                }
            }

            let contract_vault: OutputData =
                read_and_decode(contract_vault_instantiation_file).await?;

            let component_operator: OutputData =
                read_and_decode(component_operator_cid_file).await?;

            let component_aggregator: OutputData =
                read_and_decode(component_aggregator_cid_file).await?;

            #[derive(Debug, serde::Deserialize)]
            struct MiddlewareInstantiation {
                #[serde(rename = "registry_address")]
                pub _registry_address: String,
                pub service_manager_address: String,
            }

            let middleware_instantiation: MiddlewareInstantiation =
                read_and_decode(middleware_instantiation_file).await?;

            let trigger = wavs_types::Trigger::Cron {
                schedule: cron_schedule,
                start_time: None,
                end_time: None,
            };

            // Extract data based on variants
            let (operator_component, aggregator_component, _vault_address) =
                match (&component_operator, &component_aggregator, &contract_vault) {
                    (
                        OutputData::ComponentUpload {
                            kind: _,
                            digest: operator_digest,
                            cid: _,
                            uri: _,
                            gateway_url: operator_gateway_url,
                        },
                        OutputData::ComponentUpload {
                            kind: _,
                            digest: aggregator_digest,
                            cid: _,
                            uri: _,
                            gateway_url: aggregator_gateway_url,
                        },
                        OutputData::ContractInstantiate {
                            kind: _,
                            address,
                            tx_hash: _,
                        },
                    ) => {
                        let operator_component = wavs_types::Component {
                            source: wavs_types::ComponentSource::Download {
                                uri: operator_gateway_url.parse()?,
                                digest: operator_digest.clone(),
                            },
                            permissions: wavs_types::Permissions {
                                allowed_http_hosts: wavs_types::AllowedHostPermission::All,
                                file_system: false,
                            },
                            fuel_limit: None,
                            time_limit_seconds: None,
                            config: [
                                ("chain".to_string(), args.chain.to_string()),
                                ("address".to_string(), address.clone()),
                                (
                                    "trade_strategy".to_string(),
                                    serde_json::to_string(&TradeStrategy::AI)?,
                                ),
                            ]
                            .into_iter()
                            .collect(),
                            env_keys: [
                                "WAVS_ENV_COINGECKO_API_KEY".to_string(),
                                "WAVS_ENV_SKIP_API_KEY".to_string(),
                            ]
                            .into_iter()
                            .collect(),
                        };

                        let aggregator_component = wavs_types::Component {
                            source: wavs_types::ComponentSource::Download {
                                uri: aggregator_gateway_url.parse()?,
                                digest: aggregator_digest.clone(),
                            },
                            permissions: wavs_types::Permissions {
                                allowed_http_hosts: wavs_types::AllowedHostPermission::All,
                                file_system: false,
                            },
                            fuel_limit: None,
                            time_limit_seconds: None,
                            config: [
                                ("chain".to_string(), args.chain.to_string()),
                                ("address".to_string(), address.clone()),
                            ]
                            .into_iter()
                            .collect(),
                            env_keys: Default::default(),
                        };

                        (operator_component, aggregator_component, address.clone())
                    }
                    _ => return Err(anyhow::anyhow!("Invalid output data format")),
                };

            let submit = wavs_types::Submit::Aggregator {
                url: aggregator_url.to_string(),
                component: Box::new(aggregator_component),
                signature_kind: wavs_types::SignatureKind::evm_default(),
            };

            let workflow = wavs_types::Workflow {
                trigger,
                component: operator_component,
                submit,
            };

            let service = wavs_types::Service {
                name: "AI Portfolio Vault".to_string(),
                workflows: [("update_prices".parse().unwrap(), workflow)]
                    .into_iter()
                    .collect(),
                status: wavs_types::ServiceStatus::Active,
                manager: wavs_types::ServiceManager::Cosmos {
                    chain: args.chain.clone(),
                    address: middleware_instantiation.service_manager_address.parse()?,
                },
            };

            let bytes = serde_json::to_vec_pretty(&service)?;

            let digest = wavs_types::ServiceDigest::hash(&bytes);

            let resp = IpfsFile::upload(
                bytes,
                "service.json",
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
                .write(OutputData::ServiceUpload {
                    service,
                    digest,
                    cid,
                    uri: uri.clone(),
                    gateway_url: gateway_url.clone(),
                })
                .await?;

            println!("\nService URI: {}", uri);
            println!("Service Gateway URL: {}\n", gateway_url);

            Ok(())
        }
        CliCommand::AggregatorRegisterService {
            service_manager_address,
            aggregator_url,
            args,
        } => {
            println!(
                "Registering service manager {} with aggregator at {}",
                service_manager_address, aggregator_url
            );

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
                println!(
                    "Registration response: {}",
                    serde_json::to_string_pretty(&response_json)?
                );
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
            println!(
                "Adding service manager {} to WAVS operator at {}",
                service_manager_address, wavs_url
            );

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
                println!(
                    "Service configuration: {}",
                    serde_json::to_string_pretty(&response_json)?
                );
            } else {
                let error_text = response.text().await?;
                return Err(anyhow::anyhow!("Service addition failed: {}", error_text));
            }
            Ok(())
        }
    }
}
