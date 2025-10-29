use anyhow::Result;
use layer_climb::prelude::*;

use crate::config::load_chain_configs_from_wavs;

// set in start_neutron.sh
const NEUTRON_FAUCET_MNEMONIC: &str = "banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass";

static NEUTRON_FAUCET_CLIENT: tokio::sync::OnceCell<SigningClient> =
    tokio::sync::OnceCell::const_new();

const TAP_AMOUNT: u128 = 1_000_000_000;

pub async fn tap(addr: &Address, amount: Option<u128>, denom: Option<&str>) -> Result<()> {
    let signer = NEUTRON_FAUCET_CLIENT.get_or_init(create_client).await;

    signer
        .transfer(amount.unwrap_or(TAP_AMOUNT), addr, denom, None)
        .await?;

    Ok(())

    // HTTP faucet would look like:
    //
    // #[derive(Serialize)]
    // pub struct TapRequest<'a> {
    //     pub address: String,
    //     pub denom: &'a str,
    // }

    // reqwest::Client::new()
    //     .post(faucet_url.unwrap_or("http://localhost:8001/credit"))
    //     .json(&TapRequest {
    //         address: addr.to_string(),
    //         denom,
    //     })
    //     .send()
    //     .await?
    //     .error_for_status()?;
}

async fn create_client() -> SigningClient {
    let chain_configs = load_chain_configs_from_wavs(None as Option<std::path::PathBuf>)
        .await
        .unwrap();

    let chain_config = chain_configs
        .get_chain(&"cosmos:neutron-fork-1".parse().unwrap())
        .unwrap()
        .to_cosmos_config()
        .unwrap();

    let signer = KeySigner::new_mnemonic_str(NEUTRON_FAUCET_MNEMONIC, None).unwrap();

    SigningClient::new(chain_config.into(), signer, None)
        .await
        .unwrap()
}
