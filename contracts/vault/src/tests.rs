use cosmwasm_std::{coin, coins, Addr, Coin, Decimal256, Empty, Event, StdError, Uint256};
use cw_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};
use cw_ownable::Ownership;
use std::str::FromStr;

use crate::{
    astroport::SwapOperations,
    execute, instantiate,
    msg::{ExecuteMsg, InstantiateMsg, PriceUpdate, QueryMsg, VaultExecuteMsg, VaultQueryMsg},
    query,
    state::DepositState,
};

const DENOM_ATOM: &str = "uatom";
const DENOM_OSMO: &str = "uosmo";
const DENOM_UNLISTED: &str = "utoken";
const SERVICE_MANAGER: &str = "service-manager";
const ASTROPORT_ROUTER_ADDR: &str = "astroport-router";

#[derive(Clone)]
struct TestAddrs {
    owner: Addr,
    user1: Addr,
    user2: Addr,
    service_manager: Addr,
    astroport_router: Addr,
}

fn mock_app_with_addrs() -> (App, TestAddrs) {
    let mut owner_addr: Option<Addr> = None;
    let mut user1_addr: Option<Addr> = None;
    let mut user2_addr: Option<Addr> = None;
    let mut service_manager_addr: Option<Addr> = None;
    let mut astroport_router_addr: Option<Addr> = None;

    let app = AppBuilder::new().build(|router, api, storage| {
        let owner = api.addr_make(OWNER);
        let user1 = api.addr_make(USER1);
        let user2 = api.addr_make(USER2);
        let service_manager = api.addr_make(SERVICE_MANAGER);
        let astroport_router = api.addr_make(ASTROPORT_ROUTER_ADDR);

        owner_addr = Some(owner.clone());
        user1_addr = Some(user1.clone());
        user2_addr = Some(user2.clone());
        service_manager_addr = Some(service_manager.clone());
        astroport_router_addr = Some(astroport_router.clone());

        router
            .bank
            .init_balance(
                storage,
                &owner,
                vec![
                    coin(1000, DENOM_ATOM),
                    coin(1000, DENOM_OSMO),
                    coin(500, DENOM_UNLISTED),
                ],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &user1,
                vec![
                    coin(1000, DENOM_ATOM),
                    coin(1000, DENOM_OSMO),
                    coin(500, DENOM_UNLISTED),
                ],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &user2,
                vec![
                    coin(1000, DENOM_ATOM),
                    coin(1000, DENOM_OSMO),
                    coin(500, DENOM_UNLISTED),
                ],
            )
            .unwrap();
    });

    let addrs = TestAddrs {
        owner: owner_addr.expect("owner address initialized"),
        user1: user1_addr.expect("user1 address initialized"),
        user2: user2_addr.expect("user2 address initialized"),
        service_manager: service_manager_addr.expect("service manager address initialized"),
        astroport_router: astroport_router_addr.expect("astroport router address initialized"),
    };

    (app, addrs)
}

fn event_matches(event: &Event, expected_type: &str) -> bool {
    event.ty == expected_type || event.ty == format!("wasm-{expected_type}")
}

fn find_event<'a>(events: &'a [Event], expected_type: &str) -> Option<&'a Event> {
    events
        .iter()
        .find(|event| event_matches(event, expected_type))
}

fn find_event_with_attr<'a>(
    events: &'a [Event],
    expected_type: &str,
    key: &str,
    value: &str,
) -> Option<&'a Event> {
    events.iter().find(|event| {
        event_matches(event, expected_type)
            && event
                .attributes
                .iter()
                .any(|attr| attr.key == key && attr.value == value)
    })
}

fn event_attr<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    event
        .attributes
        .iter()
        .find(|attr| attr.key == key)
        .map(|attr| attr.value.as_str())
}

fn assert_error_line(err: &StdError, expected_line: &str) {
    let rendered = err.to_string();
    let last_line = rendered.lines().last().unwrap_or("").trim();
    assert_eq!(last_line, expected_line);
}

fn decimal(value: u128) -> Decimal256 {
    Decimal256::from_atomics(value, 0).expect("value fits into Decimal256")
}

fn execute_update_prices(
    app: &mut App,
    vault_addr: &Addr,
    prices: Vec<PriceUpdate>,
    swap_operations: Option<Vec<SwapOperations>>,
) -> AppResponse {
    app.execute_contract(
        vault_addr.clone(),
        vault_addr.clone(),
        &ExecuteMsg::Vault(VaultExecuteMsg::UpdatePrices {
            prices,
            swap_operations,
        }),
        &[],
    )
    .unwrap()
}

pub const OWNER: &str = "owner";
pub const USER1: &str = "user1";
pub const USER2: &str = "user2";

pub fn vault_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

fn proper_instantiate() -> (App, Addr, TestAddrs) {
    let (mut app, addrs) = mock_app_with_addrs();
    let vault_code_id = app.store_code(vault_contract());

    let msg = InstantiateMsg {
        service_manager: addrs.service_manager.to_string(),
        initial_whitelisted_denoms: vec![DENOM_ATOM.to_string(), DENOM_OSMO.to_string()],
        astroport_router: addrs.astroport_router.to_string(),
    };

    let vault_addr = app
        .instantiate_contract(vault_code_id, addrs.owner.clone(), &msg, &[], "Vault", None)
        .unwrap();

    (app, vault_addr, addrs)
}

#[test]
fn test_instantiate() {
    let (mut app, addrs) = mock_app_with_addrs();
    let vault_code_id = app.store_code(vault_contract());

    let msg = InstantiateMsg {
        service_manager: addrs.service_manager.to_string(),
        initial_whitelisted_denoms: vec![DENOM_ATOM.to_string(), DENOM_OSMO.to_string()],
        astroport_router: addrs.astroport_router.to_string(),
    };

    let vault_addr = app
        .instantiate_contract(vault_code_id, addrs.owner.clone(), &msg, &[], "Vault", None)
        .unwrap();

    // Check that contract was instantiated
    let contract_data = app.contract_data(&vault_addr).unwrap();
    assert_eq!(contract_data.label, "Vault");

    // Check that whitelisted denoms were set
    let whitelist: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetWhitelistedDenoms {}),
        )
        .unwrap();
    assert_eq!(
        whitelist,
        vec![DENOM_ATOM.to_string(), DENOM_OSMO.to_string()]
    );

    let ownership: Ownership<Addr> = app
        .wrap()
        .query_wasm_smart(&vault_addr, &QueryMsg::Vault(VaultQueryMsg::Ownership {}))
        .unwrap();
    assert_eq!(ownership.owner, Some(addrs.owner));

    // Check that total shares start at zero
    let total_shares: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(total_shares, Uint256::zero());

    // Check that vault value starts at zero
    let vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value, Decimal256::zero());
}

#[test]
fn test_deposit_success() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Deposit 100 uatom
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let user1_addr = addrs.user1.clone();
    let starting_balance = app.wrap().query_balance(&user1_addr, DENOM_ATOM).unwrap();
    assert_eq!(starting_balance.amount.to_string(), "1000");
    let res = app
        .execute_contract(
            user1_addr.clone(),
            vault_addr.clone(),
            &deposit_msg,
            &coins(100, DENOM_ATOM),
        )
        .unwrap();

    let deposit_event = find_event(&res.events, "deposit").expect("deposit event not found");
    assert_eq!(event_attr(deposit_event, "deposit_id"), Some("1"));
    assert_eq!(event_attr(deposit_event, DENOM_ATOM), Some("100"));

    // Check deposit request was created
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    assert_eq!(deposit_request.user, user1_addr);
    assert_eq!(deposit_request.coins, vec![coin(100, DENOM_ATOM)]);
    assert!(matches!(deposit_request.state, DepositState::Pending));
}

#[test]
fn test_deposit_non_whitelisted_token() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to deposit non-whitelisted token
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let err = app
        .execute_contract(
            addrs.user1.clone(),
            vault_addr,
            &deposit_msg,
            &coins(100, DENOM_UNLISTED),
        )
        .unwrap_err();

    let expected_line = format!("kind: Other, error: Token not whitelisted: {DENOM_UNLISTED}");
    assert_error_line(&err, expected_line.as_str());
}

#[test]
fn test_deposit_zero_amount() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to deposit zero amount - this should fail at the bank level
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let err = app
        .execute_contract(
            addrs.user1.clone(),
            vault_addr,
            &deposit_msg,
            &coins(0, DENOM_ATOM),
        )
        .unwrap_err();

    assert_error_line(
        &err,
        "kind: Other, error: Cannot transfer empty coins amount",
    );
}

#[test]
fn test_multi_denom_deposit_success() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Deposit multiple denoms at once: 100 uatom + 50 uosmo
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let user1_addr = addrs.user1.clone();

    let res = app
        .execute_contract(
            user1_addr.clone(),
            vault_addr.clone(),
            &deposit_msg,
            &[coin(100, DENOM_ATOM), coin(50, DENOM_OSMO)],
        )
        .unwrap();

    // Should have 1 deposit event with multiple coins
    let deposit_events: Vec<_> = res
        .events
        .iter()
        .filter(|e| event_matches(e, "deposit"))
        .collect();
    assert_eq!(deposit_events.len(), 1);

    let deposit_event = deposit_events.first().unwrap();
    assert_eq!(event_attr(deposit_event, "deposit_id"), Some("1"));
    assert_eq!(event_attr(deposit_event, DENOM_ATOM), Some("100"));
    assert_eq!(event_attr(deposit_event, DENOM_OSMO), Some("50"));

    // Check that single deposit request was created with both coins
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    assert_eq!(deposit_request.user, user1_addr);
    assert_eq!(deposit_request.coins.len(), 2);
    assert!(deposit_request.coins.contains(&coin(100, DENOM_ATOM)));
    assert!(deposit_request.coins.contains(&coin(50, DENOM_OSMO)));
    assert!(matches!(deposit_request.state, DepositState::Pending));
}

#[test]
fn test_multi_denom_deposit_with_non_whitelisted_token() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to deposit with a non-whitelisted token mixed with whitelisted ones
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let err = app
        .execute_contract(
            addrs.user1.clone(),
            vault_addr,
            &deposit_msg,
            &[coin(100, DENOM_ATOM), coin(50, DENOM_UNLISTED)],
        )
        .unwrap_err();

    let expected_line = format!("kind: Other, error: Token not whitelisted: {DENOM_UNLISTED}");
    assert_error_line(&err, expected_line.as_str());
}

#[test]
fn test_multi_denom_deposit_zero_amount_mixed() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Deposit with mixed zero and non-zero amounts
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let res = app
        .execute_contract(
            addrs.user1.clone(),
            vault_addr.clone(),
            &deposit_msg,
            &[
                coin(100, DENOM_ATOM),
                coin(0, DENOM_OSMO),
                coin(50, DENOM_OSMO),
            ],
        )
        .unwrap();

    // Should have 1 deposit event (zero amounts filtered out)
    let deposit_events: Vec<_> = res
        .events
        .iter()
        .filter(|e| event_matches(e, "deposit"))
        .collect();
    assert_eq!(deposit_events.len(), 1);

    let deposit_event = deposit_events.first().unwrap();
    assert_eq!(event_attr(deposit_event, DENOM_ATOM), Some("100"));
    assert_eq!(event_attr(deposit_event, DENOM_OSMO), Some("50"));

    // Check that single deposit request was created with only non-zero coins
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    assert_eq!(deposit_request.coins.len(), 2);
    assert!(deposit_request.coins.contains(&coin(100, DENOM_ATOM)));
    assert!(deposit_request.coins.contains(&coin(50, DENOM_OSMO)));
    assert!(!deposit_request.coins.iter().any(|c| c.amount.is_zero()));
}

#[test]
fn test_multi_denom_deposit_no_funds() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to deposit with no funds
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let err = app
        .execute_contract(addrs.user1.clone(), vault_addr, &deposit_msg, &[])
        .unwrap_err();

    assert_error_line(&err, "kind: Other, error: No funds provided");
}

#[test]
fn test_multi_denom_deposit_only_zero_amounts() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to deposit with only zero amounts - this should fail at the bank level
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let err = app
        .execute_contract(
            addrs.user1.clone(),
            vault_addr,
            &deposit_msg,
            &[coin(0, DENOM_ATOM), coin(0, DENOM_OSMO)],
        )
        .unwrap_err();

    assert_error_line(
        &err,
        "kind: Other, error: Cannot transfer empty coins amount",
    );
}

#[test]
fn test_multi_denom_deposit_price_processing() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Make a multi-denom deposit
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &[coin(100, DENOM_ATOM), coin(200, DENOM_OSMO)],
    )
    .unwrap();

    // Update prices - should process the single multi-denom deposit
    let res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![
            PriceUpdate {
                denom: DENOM_ATOM.to_string(),
                price_usd: decimal(10),
            },
            PriceUpdate {
                denom: DENOM_OSMO.to_string(),
                price_usd: decimal(5),
            },
        ],
        None,
    );

    // Should have processed 1 deposit (containing multiple coins)
    let processed_events: Vec<_> = res
        .events
        .iter()
        .filter(|e| event_matches(e, "deposit_processed"))
        .collect();
    assert_eq!(processed_events.len(), 1);

    let deposit_event = processed_events.first().unwrap();
    assert_eq!(event_attr(deposit_event, "deposit_id"), Some("1"));

    // Check vault value includes both deposits: (100 * 10) + (200 * 5) = 1000 + 1000 = 2000
    let vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value, decimal(2000));

    // Check vault assets include both denoms
    let vault_assets: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultAssets {}),
        )
        .unwrap();
    assert_eq!(vault_assets.len(), 2);
    assert!(vault_assets.contains(&coin(100, DENOM_ATOM)));
    assert!(vault_assets.contains(&coin(200, DENOM_OSMO)));

    // Check that the deposit was completed with the correct total value
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    match deposit_request.state {
        crate::state::DepositState::Completed { value_usd } => {
            assert_eq!(value_usd, decimal(2000));
        }
        _ => panic!("Deposit should be completed"),
    }
}

#[test]
fn test_update_prices_and_process_deposits() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // First make a deposit
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    // Update prices - this should process the pending deposit
    let res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    let expected_price = decimal(10).to_string();
    let price_event = find_event_with_attr(&res.events, "price_updated", "denom", DENOM_ATOM)
        .expect("price update event not found");
    assert_eq!(
        event_attr(price_event, "price_usd"),
        Some(expected_price.as_str())
    );

    let deposit_event = find_event_with_attr(&res.events, "deposit_processed", "deposit_id", "1")
        .expect("deposit processed event not found");
    let expected_value = decimal(1000).to_string();
    assert_eq!(
        event_attr(deposit_event, "value_usd"),
        Some(expected_value.as_str())
    );
    let issued_shares =
        event_attr(deposit_event, "shares_issued").expect("shares_issued attribute missing");
    let issued_shares = Uint256::from_str(issued_shares).expect("shares_issued parses as Uint256");
    assert!(issued_shares > Uint256::zero());

    // Check that deposit was processed
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    match deposit_request.state {
        DepositState::Completed { value_usd } => assert_eq!(value_usd, decimal(1000)),
        _ => panic!("Deposit should be completed"),
    }

    // Check that user received shares
    let total_shares: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert!(total_shares > Uint256::zero());

    // Check vault value
    let vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value, Decimal256::from_atomics(1000u128, 0).unwrap());
}

#[test]
fn test_share_issuance_precision() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});

    // First deposit: 100 uatom at $10 should mint 1,000,000,000 shares.
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    let first_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    let deposit_one_event =
        find_event_with_attr(&first_res.events, "deposit_processed", "deposit_id", "1")
            .expect("missing deposit_processed event for deposit 1");
    assert_eq!(
        event_attr(deposit_one_event, "value_usd"),
        Some(decimal(1000).to_string().as_str())
    );
    let minted_one = Uint256::from_str(
        event_attr(deposit_one_event, "shares_issued")
            .expect("shares_issued missing for deposit 1"),
    )
    .expect("shares_issued parses for deposit 1");
    let expected_first = Uint256::from(1_000_000_000u128);
    assert_eq!(minted_one, expected_first);

    let total_shares_after_first: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(total_shares_after_first, expected_first);

    // Second deposit: 50 uatom at same $10 price should mint proportional shares.
    app.execute_contract(
        addrs.user2.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(50, DENOM_ATOM),
    )
    .unwrap();

    let second_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    let deposit_two_event =
        find_event_with_attr(&second_res.events, "deposit_processed", "deposit_id", "2")
            .expect("missing deposit_processed event for deposit 2");
    assert_eq!(
        event_attr(deposit_two_event, "value_usd"),
        Some(decimal(500).to_string().as_str())
    );
    let minted_two = Uint256::from_str(
        event_attr(deposit_two_event, "shares_issued")
            .expect("shares_issued missing for deposit 2"),
    )
    .expect("shares_issued parses for deposit 2");
    let expected_second = Decimal256::from_atomics(expected_first, 0)
        .expect("convert total shares to decimal")
        .checked_mul(decimal(500))
        .expect("multiply by value")
        .checked_div(decimal(1000))
        .expect("divide by vault value")
        .to_uint_ceil();
    assert_eq!(minted_two, expected_second);

    let total_shares_after_second: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(
        total_shares_after_second,
        expected_first
            .checked_add(expected_second)
            .expect("add shares")
    );

    for deposit_id in 1..=2 {
        let deposit: crate::state::DepositRequest = app
            .wrap()
            .query_wasm_smart(
                &vault_addr,
                &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id }),
            )
            .unwrap();
        match deposit.state {
            DepositState::Completed { .. } => {}
            _ => panic!("deposit {deposit_id} should be completed"),
        }
    }
}

#[test]
fn test_update_prices_rejects_zero_price() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Create a pending deposit so we can assert it remains untouched.
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    let err = app
        .execute_contract(
            vault_addr.clone(),
            vault_addr.clone(),
            &ExecuteMsg::Vault(VaultExecuteMsg::UpdatePrices {
                prices: vec![PriceUpdate {
                    denom: DENOM_ATOM.to_string(),
                    price_usd: Decimal256::zero(),
                }],
                swap_operations: None,
            }),
            &[],
        )
        .unwrap_err();

    let expected_line =
        format!("kind: Other, error: Price must be greater than zero for denom: {DENOM_ATOM}");
    assert_error_line(&err, expected_line.as_str());

    // Ensure deposit stayed pending
    let deposit_request = app
        .wrap()
        .query_wasm_smart::<crate::state::DepositRequest>(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 1 }),
        )
        .unwrap();
    assert!(matches!(deposit_request.state, DepositState::Pending));
}

#[test]
fn test_multi_denom_price_updates_with_pending_handling() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});

    // Create two pending deposits across different denoms.
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();
    app.execute_contract(
        addrs.user2.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(50, DENOM_OSMO),
    )
    .unwrap();

    // First price update only covers uatom, leaving the uosmo deposit pending.
    let first_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    assert!(
        find_event_with_attr(&first_res.events, "price_updated", "denom", DENOM_ATOM).is_some(),
        "expected price update for uatom"
    );
    find_event_with_attr(&first_res.events, "deposit_processed", "deposit_id", "1")
        .expect("deposit 1 should process after uatom price update");
    assert!(
        find_event_with_attr(&first_res.events, "deposit_processed", "deposit_id", "2").is_none(),
        "deposit 2 should remain pending without a price"
    );

    let deposit_two_pending: crate::state::DepositRequest = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 2 }),
        )
        .unwrap();
    assert!(matches!(deposit_two_pending.state, DepositState::Pending));

    let total_shares_after_first: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    let expected_first = Uint256::from(1_000_000_000u128);
    assert_eq!(total_shares_after_first, expected_first);

    let vault_value_before_second: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value_before_second, decimal(1000));

    // Second price update supplies both denom prices, processing the second deposit.
    let second_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![
            PriceUpdate {
                denom: DENOM_ATOM.to_string(),
                price_usd: decimal(10),
            },
            PriceUpdate {
                denom: DENOM_OSMO.to_string(),
                price_usd: decimal(5),
            },
        ],
        None,
    );

    assert!(
        find_event_with_attr(&second_res.events, "price_updated", "denom", DENOM_ATOM).is_some(),
        "expected price update for uatom"
    );
    assert!(
        find_event_with_attr(&second_res.events, "price_updated", "denom", DENOM_OSMO).is_some(),
        "expected price update for uosmo"
    );

    let deposit_two_event =
        find_event_with_attr(&second_res.events, "deposit_processed", "deposit_id", "2")
            .expect("deposit 2 should process when price exists");
    assert_eq!(
        event_attr(deposit_two_event, "value_usd"),
        Some(decimal(250).to_string().as_str())
    );
    let minted_two = Uint256::from_str(
        event_attr(deposit_two_event, "shares_issued")
            .expect("shares_issued missing for deposit 2"),
    )
    .expect("shares_issued parses for deposit 2");
    let expected_second = Decimal256::from_atomics(expected_first, 0)
        .expect("convert total shares to decimal")
        .checked_mul(decimal(250))
        .expect("multiply by value")
        .checked_div(decimal(1000))
        .expect("divide by vault value")
        .to_uint_ceil();
    assert_eq!(minted_two, expected_second);

    let total_shares_after_second: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(
        total_shares_after_second,
        expected_first
            .checked_add(expected_second)
            .expect("add shares"),
    );

    let deposit_two_completed: crate::state::DepositRequest = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id: 2 }),
        )
        .unwrap();
    match deposit_two_completed.state {
        DepositState::Completed { value_usd } => assert_eq!(value_usd, decimal(250)),
        _ => panic!("deposit 2 should be completed after second price update"),
    }

    let final_vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(final_vault_value, decimal(1250));
}

#[test]
fn test_price_volatility_updates() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Initial deposit processed at $10 per uatom.
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    let initial_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );
    let initial_event =
        find_event_with_attr(&initial_res.events, "deposit_processed", "deposit_id", "1")
            .expect("deposit should process with initial price update");
    assert_eq!(
        event_attr(initial_event, "value_usd"),
        Some(decimal(1000).to_string().as_str())
    );
    let initial_shares = Uint256::from_str(
        event_attr(initial_event, "shares_issued")
            .expect("shares_issued missing on initial deposit"),
    )
    .expect("shares_issued parses for initial deposit");

    let total_shares_after_initial: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(total_shares_after_initial, initial_shares);

    let vault_value_after_initial: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value_after_initial, decimal(1000));

    // Price drops to $5; shares stay constant but vault USD value halves.
    let volatility_res = execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(5),
        }],
        None,
    );

    let price_event =
        find_event_with_attr(&volatility_res.events, "price_updated", "denom", DENOM_ATOM)
            .expect("price update event missing");
    assert_eq!(
        event_attr(price_event, "price_usd"),
        Some(decimal(5).to_string().as_str())
    );
    assert!(
        find_event_with_attr(
            &volatility_res.events,
            "deposit_processed",
            "deposit_id",
            "1"
        )
        .is_none(),
        "no new deposits should process during pure price update"
    );

    let total_shares_after_drop: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(total_shares_after_drop, initial_shares);

    let vault_value_after_drop: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value_after_drop, decimal(500));

    // Withdrawal still returns the full 100 uatom even after price drop.
    let withdraw_msg = ExecuteMsg::Vault(VaultExecuteMsg::Withdraw {
        shares: initial_shares,
    });
    let balance_before = app.wrap().query_balance(&addrs.user1, DENOM_ATOM).unwrap();

    app.execute_contract(addrs.user1.clone(), vault_addr.clone(), &withdraw_msg, &[])
        .unwrap();

    let balance_after = app.wrap().query_balance(&addrs.user1, DENOM_ATOM).unwrap();
    assert_eq!(
        balance_after.amount,
        balance_before
            .amount
            .checked_add(Uint256::from(100u128))
            .expect("add amounts")
    );

    let final_vault_assets: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultAssets {}),
        )
        .unwrap();
    assert!(
        final_vault_assets.iter().all(|coin| coin.amount.is_zero()),
        "all vault asset balances should be zero after full withdrawal"
    );

    let final_total_shares: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert_eq!(final_total_shares, Uint256::zero());

    let final_vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(final_vault_value, Decimal256::zero());
}

#[test]
fn test_withdraw_success() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    let user1_addr = addrs.user1.clone();
    app.execute_contract(
        user1_addr.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    let total_shares_before: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert!(total_shares_before > Uint256::zero());

    let vault_value_before: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();

    let withdraw_shares = total_shares_before * Uint256::from(1u128) / Uint256::from(2u128);
    let withdraw_msg = ExecuteMsg::Vault(VaultExecuteMsg::Withdraw {
        shares: withdraw_shares,
    });
    let withdraw_shares_str = withdraw_shares.to_string();

    let user_balance_before = app.wrap().query_balance(&user1_addr, DENOM_ATOM).unwrap();

    let res = app
        .execute_contract(user1_addr.clone(), vault_addr.clone(), &withdraw_msg, &[])
        .unwrap();

    let user_balance_after = app.wrap().query_balance(&user1_addr, DENOM_ATOM).unwrap();
    assert!(
        user_balance_after.amount > user_balance_before.amount,
        "withdrawal should increase user balance"
    );

    let total_shares_after: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    assert!(total_shares_after < total_shares_before);

    let vault_value_after: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert!(vault_value_after < vault_value_before);

    let wasm_event = find_event_with_attr(&res.events, "wasm", "method", "withdraw")
        .expect("withdraw event missing");
    assert_eq!(event_attr(wasm_event, "user"), Some(user1_addr.as_str()));
    assert_eq!(
        event_attr(wasm_event, "shares"),
        Some(withdraw_shares_str.as_str())
    );
    assert!(
        find_event(&res.events, "transfer").is_some(),
        "bank transfer event should be present"
    );
}

#[test]
fn test_withdraw_zero_shares() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    let withdraw_msg = ExecuteMsg::Vault(VaultExecuteMsg::Withdraw {
        shares: Uint256::zero(),
    });

    let err = app
        .execute_contract(addrs.user1.clone(), vault_addr, &withdraw_msg, &[])
        .unwrap_err();

    assert_error_line(&err, "kind: Other, error: Cannot withdraw zero shares");
}

#[test]
fn test_withdraw_insufficient_shares() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to withdraw without having any shares
    let withdraw_msg = ExecuteMsg::Vault(VaultExecuteMsg::Withdraw {
        shares: Uint256::from(100u128),
    });

    let err = app
        .execute_contract(addrs.user1.clone(), vault_addr, &withdraw_msg, &[])
        .unwrap_err();

    assert_error_line(&err, "kind: Other, error: Insufficient shares");
}

#[test]
fn test_update_whitelist() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Add new token to whitelist
    let add_msg = ExecuteMsg::Vault(VaultExecuteMsg::UpdateWhitelist {
        to_add: Some(vec!["uwasm".to_string()]),
        to_remove: None,
    });

    let add_res = app
        .execute_contract(addrs.owner.clone(), vault_addr.clone(), &add_msg, &[])
        .unwrap();
    let add_event = find_event_with_attr(&add_res.events, "wasm", "method", "update_whitelist")
        .expect("update whitelist event missing");
    assert_eq!(event_attr(add_event, "tokens_added"), Some("1"));
    assert_eq!(event_attr(add_event, "tokens_removed"), Some("0"));

    // Check that token was added
    let whitelist: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetWhitelistedDenoms {}),
        )
        .unwrap();
    assert!(whitelist.contains(&"uwasm".to_string()));

    // Remove token from whitelist
    let remove_msg = ExecuteMsg::Vault(VaultExecuteMsg::UpdateWhitelist {
        to_add: None,
        to_remove: Some(vec!["uwasm".to_string()]),
    });

    let remove_res = app
        .execute_contract(addrs.owner.clone(), vault_addr.clone(), &remove_msg, &[])
        .unwrap();
    let remove_event =
        find_event_with_attr(&remove_res.events, "wasm", "method", "update_whitelist")
            .expect("update whitelist event missing on removal");
    assert_eq!(event_attr(remove_event, "tokens_added"), Some("0"));
    assert_eq!(event_attr(remove_event, "tokens_removed"), Some("1"));

    // Check that token was removed
    let whitelist: Vec<String> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetWhitelistedDenoms {}),
        )
        .unwrap();
    assert!(!whitelist.contains(&"uwasm".to_string()));
}

#[test]
fn test_update_whitelist_unauthorized() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Try to update whitelist as non-owner
    let update_msg = ExecuteMsg::Vault(VaultExecuteMsg::UpdateWhitelist {
        to_add: Some(vec!["uwasm".to_string()]),
        to_remove: None,
    });

    let err = app
        .execute_contract(addrs.user1.clone(), vault_addr, &update_msg, &[])
        .unwrap_err();

    assert_error_line(
        &err,
        "kind: Other, error: Caller is not the contract's current owner",
    );
}

#[test]
fn test_multiple_deposits_and_withdrawals() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    app.execute_contract(
        addrs.user2.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(200, DENOM_ATOM),
    )
    .unwrap();

    execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    // Check total vault value
    let vault_value: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultValue {}),
        )
        .unwrap();
    assert_eq!(vault_value, decimal(3000));

    for deposit_id in 1..=2 {
        let deposit_request: crate::state::DepositRequest = app
            .wrap()
            .query_wasm_smart(
                &vault_addr,
                &QueryMsg::Vault(VaultQueryMsg::GetDepositRequest { deposit_id }),
            )
            .unwrap();
        assert!(matches!(
            deposit_request.state,
            DepositState::Completed { .. }
        ));
    }

    // User1 withdraws all their shares (approximately 1/3 of total since User1 deposited 100, User2 deposited 200)
    let total_shares: Uint256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetTotalShares {}),
        )
        .unwrap();
    let user1_shares = total_shares * Uint256::from(1u128) / Uint256::from(3u128); // User1 deposited 1/3 of total

    let withdraw_msg = ExecuteMsg::Vault(VaultExecuteMsg::Withdraw {
        shares: user1_shares,
    });

    let user1_balance_before = app.wrap().query_balance(&addrs.user1, DENOM_ATOM).unwrap();

    app.execute_contract(addrs.user1.clone(), vault_addr.clone(), &withdraw_msg, &[])
        .unwrap();

    let user1_balance_after = app.wrap().query_balance(&addrs.user1, DENOM_ATOM).unwrap();
    assert!(user1_balance_after.amount > user1_balance_before.amount);
}

#[test]
fn test_vault_assets_query() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Initially no assets
    let vault_assets: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultAssets {}),
        )
        .unwrap();
    assert!(vault_assets.is_empty());

    // Make a deposit
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    // Update prices to process deposit
    execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    // Check vault assets
    let vault_assets: Vec<Coin> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetVaultAssets {}),
        )
        .unwrap();
    assert_eq!(vault_assets.len(), 1);
    assert_eq!(vault_assets[0], coin(100, DENOM_ATOM));
}

#[test]
fn test_price_query() {
    let (mut app, vault_addr, _) = proper_instantiate();

    // Initially no price
    let price: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetPrice {
                denom: DENOM_ATOM.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(price, Decimal256::zero());

    // Update price
    execute_update_prices(
        &mut app,
        &vault_addr,
        vec![PriceUpdate {
            denom: DENOM_ATOM.to_string(),
            price_usd: decimal(10),
        }],
        None,
    );

    // Check price
    let price: Decimal256 = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::GetPrice {
                denom: DENOM_ATOM.to_string(),
            }),
        )
        .unwrap();
    assert_eq!(price, decimal(10));
}

#[test]
fn test_list_deposit_requests() {
    let (mut app, vault_addr, addrs) = proper_instantiate();

    // Make multiple deposits
    let deposit_msg = ExecuteMsg::Vault(VaultExecuteMsg::Deposit {});
    app.execute_contract(
        addrs.user1.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(100, DENOM_ATOM),
    )
    .unwrap();

    app.execute_contract(
        addrs.user2.clone(),
        vault_addr.clone(),
        &deposit_msg,
        &coins(200, DENOM_ATOM),
    )
    .unwrap();

    // List all deposit requests
    let deposits: Vec<crate::state::DepositRequest> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::ListDepositRequests {
                start_after: None,
                limit: None,
            }),
        )
        .unwrap();
    assert_eq!(deposits.len(), 2);
    assert_eq!(deposits[0].id, 1);
    assert!(matches!(deposits[0].state, DepositState::Pending));
    assert_eq!(deposits[1].id, 2);
    assert!(matches!(deposits[1].state, DepositState::Pending));

    // List with pagination
    let deposits: Vec<crate::state::DepositRequest> = app
        .wrap()
        .query_wasm_smart(
            &vault_addr,
            &QueryMsg::Vault(VaultQueryMsg::ListDepositRequests {
                start_after: Some(1),
                limit: Some(1),
            }),
        )
        .unwrap();
    assert_eq!(deposits.len(), 1);
    assert_eq!(deposits[0].id, 2);
}
