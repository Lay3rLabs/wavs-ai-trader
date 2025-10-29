//! Abstraction specifically for the on-chain multi-test environment
pub mod vault;

use ai_portfolio_utils::{
    addr::AnyAddr,
    client::{AnyExecutor, AnyQuerier},
    faucet,
};
use deadpool::managed::Pool;
use rand::prelude::*;
use tokio::sync::OnceCell;

use layer_climb::{
    pool::{SigningClientPool, SigningClientPoolManager},
    prelude::*,
};

use crate::config::TestConfig;

const MAINTAIN_MINIMUM_BALANCE_THRESHOLD: u128 = 100000;
const MAINTAIN_MINIMUM_BALANCE_TOPUP: u128 = 10000000;

static TEST_POOL: OnceCell<TestPool> = OnceCell::const_new();

#[derive(Clone)]
pub struct AppClient {
    pub querier: AnyQuerier,
    pub executor: AnyExecutor,
    pub chain_config: ChainConfig,
}

impl AppClient {
    pub async fn new() -> Self {
        let TestPool { pool, .. } = TestPool::get().await;
        let chain_config = { pool.get().await.unwrap().querier.chain_config.clone() };

        Self {
            querier: pool.clone().into(),
            executor: pool.into(),
            chain_config,
        }
    }

    pub fn pool(&self) -> SigningClientPool {
        match &self.executor {
            AnyExecutor::ClimbPool(pool) => pool.clone(),
            _ => unreachable!(),
        }
    }

    // TODO - something faster, like MockApi make_addr...
    pub async fn rand_address(&self) -> AnyAddr {
        let mut rng = rand::rng();
        let entropy: [u8; 32] = rng.random();
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).unwrap().to_string();
        let signer = KeySigner::new_mnemonic_str(&mnemonic, None)
            .expect("Failed to create KeySigner from mnemonic");

        let addr = self
            .chain_config
            .address_from_pub_key(&signer.public_key().await.unwrap())
            .unwrap();

        addr.into()
    }

    pub async fn rand_signing_client(&self) -> SigningClient {
        let mut rng = rand::rng();
        let entropy: [u8; 32] = rng.random();
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).unwrap().to_string();
        let signer = KeySigner::new_mnemonic_str(&mnemonic, None)
            .expect("Failed to create KeySigner from mnemonic");

        // This needs funding first, otherwise you cannot query sequence and account number
        let signer_addr = signer.address(&self.chain_config).await.unwrap();
        faucet::tap(&signer_addr, None, None).await.unwrap();

        SigningClient::new(self.chain_config.clone(), signer, None)
            .await
            .unwrap()
    }
}

#[derive(Clone)]
pub struct TestPool {
    pub pool: SigningClientPool,
}

impl TestPool {
    pub async fn get() -> Self {
        TEST_POOL.get_or_init(TestPool::instantiate).await.clone()
    }

    async fn instantiate() -> Self {
        let mut rng = rand::rng();

        let entropy: [u8; 32] = rng.random();
        let mnemonic = bip39::Mnemonic::from_entropy(&entropy).unwrap().to_string();

        let chain_config = TestConfig::get().await.chain_config;
        let chain_config: ChainConfig = chain_config;
        let querier = QueryClient::new(chain_config.clone(), None).await.unwrap();

        // Before we run off and create the pool, make sure it has funds!
        let signer = KeySigner::new_mnemonic_str(&mnemonic, None)
            .expect("Failed to create KeySigner from mnemonic");

        let addr = chain_config
            .address_from_pub_key(&signer.public_key().await.unwrap())
            .unwrap();

        let balance = querier
            .balance(addr.clone(), None)
            .await
            .unwrap()
            .unwrap_or_default();

        if balance < 10000000000 {
            tracing::info!("{} has balance of {}, sending some funds...", addr, balance);

            faucet::tap(&addr, None, None).await.unwrap();
            let new_balance = querier
                .balance(addr, None)
                .await
                .unwrap()
                .unwrap_or_default();
            if new_balance == balance {
                panic!("Failed to tap faucet, balance did not change");
            }
            tracing::info!("new balance is {:?}", new_balance);
        } else {
            tracing::info!("{} has balance of {}, no need to tap faucet", addr, balance);
        }

        // now we can properly create the pool
        let pool =
            SigningClientPoolManager::new_mnemonic(mnemonic, chain_config.clone(), None, None)
                .with_minimum_balance(
                    MAINTAIN_MINIMUM_BALANCE_THRESHOLD,
                    MAINTAIN_MINIMUM_BALANCE_TOPUP,
                    None,
                    None,
                )
                .await
                .unwrap();

        let pool = SigningClientPool::new(Pool::builder(pool).max_size(8).build().unwrap());

        Self { pool }
    }
}
