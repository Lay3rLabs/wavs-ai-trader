//! Abstractions for different backends (Climb, Climb Pool, MultiTest)
//! Provides AnyQuerier and AnyExecutor to represent _any_ contract querier/executor
//! The idea is that by moving the heavy-lifting here, we're free to write higher-level code
//! that provides an idiomatic and clean API
pub mod vault;

use anyhow::{anyhow, Result};
#[cfg(feature = "multitest")]
use cw_multi_test::{App, Executor};
use layer_climb::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
#[cfg(feature = "multitest")]
use std::{cell::RefCell, rc::Rc};

use crate::addr::AnyAddr;

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AnyQuerier {
    Climb(QueryClient),
    #[cfg(feature = "on-chain")]
    ClimbPool(layer_climb::pool::SigningClientPool),
    #[cfg(feature = "multitest")]
    MultiTest(Rc<RefCell<App>>),
}

impl From<QueryClient> for AnyQuerier {
    fn from(client: QueryClient) -> AnyQuerier {
        AnyQuerier::Climb(client)
    }
}

#[cfg(feature = "on-chain")]
impl From<layer_climb::pool::SigningClientPool> for AnyQuerier {
    fn from(pool: layer_climb::pool::SigningClientPool) -> AnyQuerier {
        AnyQuerier::ClimbPool(pool)
    }
}

#[cfg(feature = "multitest")]
impl From<Rc<RefCell<App>>> for AnyQuerier {
    fn from(app: Rc<RefCell<App>>) -> AnyQuerier {
        AnyQuerier::MultiTest(app)
    }
}

impl AnyQuerier {
    pub async fn contract_query<
        RESP: DeserializeOwned + Send + Sync + Debug,
        MSG: Serialize + Debug,
    >(
        &self,
        address: &AnyAddr,
        msg: &MSG,
    ) -> Result<RESP> {
        match self {
            Self::Climb(client) => client.contract_smart(&address.into(), msg).await,
            #[cfg(feature = "on-chain")]
            Self::ClimbPool(pool) => {
                let client = pool.get().await.map_err(|e| anyhow!("{e:?}"))?;
                client.querier.contract_smart(&address.into(), msg).await
            }
            #[cfg(feature = "multitest")]
            Self::MultiTest(app) => Ok(app
                .borrow()
                .wrap()
                .query_wasm_smart(address.to_string(), msg)
                .map_err(|e| anyhow!("{e:?}"))?),
        }
    }
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AnyExecutor {
    Climb(SigningClient),
    #[cfg(feature = "on-chain")]
    ClimbPool(layer_climb::pool::SigningClientPool),
    #[cfg(feature = "multitest")]
    MultiTest {
        app: Rc<RefCell<App>>,
        admin: cosmwasm_std::Addr,
    },
}

impl From<SigningClient> for AnyExecutor {
    fn from(client: SigningClient) -> AnyExecutor {
        AnyExecutor::Climb(client)
    }
}

#[cfg(feature = "on-chain")]
impl From<layer_climb::pool::SigningClientPool> for AnyExecutor {
    fn from(pool: layer_climb::pool::SigningClientPool) -> AnyExecutor {
        AnyExecutor::ClimbPool(pool)
    }
}

#[cfg(feature = "multitest")]
impl From<(Rc<RefCell<App>>, cosmwasm_std::Addr)> for AnyExecutor {
    fn from((app, admin): (Rc<RefCell<App>>, cosmwasm_std::Addr)) -> AnyExecutor {
        AnyExecutor::MultiTest { app, admin }
    }
}

impl AnyExecutor {
    pub async fn contract_exec<MSG: Serialize + std::fmt::Debug>(
        &self,
        address: &AnyAddr,
        msg: &MSG,
        funds: &[cosmwasm_std::Coin],
    ) -> Result<AnyTxResponse> {
        match self {
            Self::Climb(client) => {
                let funds = funds
                    .iter()
                    .map(|c| layer_climb::prelude::Coin {
                        denom: c.denom.clone(),
                        amount: c.amount.to_string(),
                    })
                    .collect::<Vec<_>>();

                client
                    .contract_execute(&address.into(), msg, funds, None)
                    .await
                    .map(AnyTxResponse::Climb)
            }
            #[cfg(feature = "on-chain")]
            Self::ClimbPool(pool) => {
                let client = pool.get().await.map_err(|e| anyhow!("{e:?}"))?;
                let funds = funds
                    .iter()
                    .map(|c| layer_climb::prelude::Coin {
                        denom: c.denom.clone(),
                        amount: c.amount.to_string(),
                    })
                    .collect::<Vec<_>>();

                client
                    .contract_execute(&address.into(), msg, funds, None)
                    .await
                    .map(AnyTxResponse::Climb)
            }
            #[cfg(feature = "multitest")]
            Self::MultiTest { app, admin } => Ok(app
                .borrow_mut()
                .execute_contract(admin.clone(), address.into(), msg, funds)
                .map(AnyTxResponse::MultiTest)
                .map_err(|e| anyhow!("{e:?}"))?),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AnyTxResponse {
    Climb(layer_climb::proto::abci::TxResponse),
    #[cfg(feature = "multitest")]
    MultiTest(cw_multi_test::AppResponse),
}

impl<'a> From<&'a AnyTxResponse> for CosmosTxEvents<'a> {
    fn from(value: &'a AnyTxResponse) -> Self {
        match value {
            AnyTxResponse::Climb(resp) => CosmosTxEvents::from(resp),
            #[cfg(feature = "multitest")]
            AnyTxResponse::MultiTest(resp) => CosmosTxEvents::from(resp.events.as_slice()),
        }
    }
}

impl AnyTxResponse {
    pub fn unchecked_into_tx_response(self) -> layer_climb::proto::abci::TxResponse {
        match self {
            Self::Climb(tx_resp) => tx_resp,
            #[allow(unreachable_patterns)]
            _ => panic!("unable to get unchecked tx response"),
        }
    }
}
