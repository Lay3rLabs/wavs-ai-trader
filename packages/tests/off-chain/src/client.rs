//! Abstraction specifically for the off-chain multi-test environment
pub mod vault;
use std::{cell::RefCell, rc::Rc};
use ai_portfolio_utils::client::{AnyExecutor, AnyQuerier};

use cosmwasm_std::{Addr, Coin};
use cw_multi_test::App;

#[derive(Clone)]
pub struct AppClient {
    pub querier: AnyQuerier,
    pub executor: AnyExecutor,
}

impl AppClient {
    pub fn new(admin: &str) -> Self {
        let app = Rc::new(RefCell::new(App::new(|router, api, storage| {
            // Initialize multiple users with different tokens for testing
            let admin_addr = api.addr_make(admin);

            router
                .bank
                .init_balance(
                    storage,
                    &admin_addr,
                    vec![
                        Coin {
                            denom: "uatom".to_string(),
                            amount: 1_000_000_000u128.into(),
                        },
                        Coin {
                            denom: "uosmo".to_string(),
                            amount: 1_000_000_000u128.into(),
                        },
                        Coin {
                            denom: "ujuno".to_string(),
                            amount: 1_000_000_000u128.into(),
                        },
                    ],
                )
                .unwrap();

            // Initialize regular users
            let user1 = api.addr_make("user1");
            let user2 = api.addr_make("user2");
            let user3 = api.addr_make("user3");

            for user in [user1, user2, user3] {
                router
                    .bank
                    .init_balance(
                        storage,
                        &user,
                        vec![
                            Coin {
                                denom: "uatom".to_string(),
                                amount: 1_000_000_000u128.into(),
                            },
                            Coin {
                                denom: "uosmo".to_string(),
                                amount: 1_000_000_000u128.into(),
                            },
                            Coin {
                                denom: "ujuno".to_string(),
                                amount: 1_000_000_000u128.into(),
                            },
                        ],
                    )
                    .unwrap();
            }
        })));

        let admin = app.borrow().api().addr_make(admin);

        Self {
            querier: app.clone().into(),
            executor: (app.clone(), admin).into(),
        }
    }

    pub fn with_app<T>(&self, f: impl FnOnce(&App) -> T) -> T {
        match &self.executor {
            AnyExecutor::MultiTest { app, .. } => f(&app.borrow()),
            _ => unreachable!(),
        }
    }

    pub fn with_app_mut<T>(&self, f: impl FnOnce(&mut App) -> T) -> T {
        match &self.executor {
            AnyExecutor::MultiTest { app, .. } => f(&mut app.borrow_mut()),
            _ => unreachable!(),
        }
    }

    pub fn clone_app(&self) -> Rc<RefCell<App>> {
        match &self.executor {
            AnyExecutor::MultiTest { app, .. } => app.clone(),
            _ => unreachable!(),
        }
    }

    pub fn admin(&self) -> Addr {
        match &self.executor {
            AnyExecutor::MultiTest { admin, .. } => admin.clone(),
            _ => unreachable!(),
        }
    }

    /// Create a new user address
    pub fn make_user(&self, name: &str) -> Addr {
        self.with_app(|app| app.api().addr_make(name))
    }

    /// Get pre-initialized user addresses
    pub fn user1(&self) -> Addr {
        self.with_app(|app| app.api().addr_make("user1"))
    }

    pub fn user2(&self) -> Addr {
        self.with_app(|app| app.api().addr_make("user2"))
    }

    pub fn user3(&self) -> Addr {
        self.with_app(|app| app.api().addr_make("user3"))
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &Addr, denom: &str) -> u128 {
        self.with_app(|app| {
            let balance = app.wrap()
                .query_balance(address, denom)
                .unwrap();
            balance.amount.to_string().parse().unwrap()
        })
    }
}