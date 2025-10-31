use ai_portfolio_test_common::shared_tests::vault::{
    get_admin, test_vault_comprehensive_workflow as shared_test_comprehensive_workflow,
    test_vault_deposit as shared_test_deposit,
    test_vault_error_handling as shared_test_error_handling,
    test_vault_instantiation as shared_test_instantiation,
    test_vault_price_updates as shared_test_price_updates,
    test_vault_whitelist_management as shared_test_whitelist_management,
    test_vault_withdrawal as shared_test_withdrawal, VaultComprehensiveWorkflowProps,
    VaultDepositProps, VaultErrorHandlingProps, VaultInstantiationProps, VaultPriceUpdateProps,
    VaultWhitelistProps, VaultWithdrawalProps,
};
use ai_portfolio_utils::tracing::tracing_init;
use cosmwasm_std::{coin, coins, Decimal256, Uint256};
use off_chain_tests::client::{vault::VaultClient, AppClient};
use std::str::FromStr;

#[tokio::test]
async fn test_vault_initialization() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());

    // Check that ownership is set correctly
    get_admin(&vault.querier, app_client.admin().as_ref()).await;

    // Test vault initialization with shared test logic
    shared_test_instantiation(
        &vault.querier,
        VaultInstantiationProps {
            service_manager: app_client.admin().to_string(),
            initial_whitelisted_denoms: vec![
                "uatom".to_string(),
                "uosmo".to_string(),
                "ujuno".to_string(),
            ],
            skip_entry_point: app_client.admin().to_string(),
        },
    )
    .await;
}

#[tokio::test]
async fn test_vault_single_deposit() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));

    // Test single deposit with shared test logic
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user1.into(),
            deposit_amount: coins(100_000_000, "uatom"),
        },
    )
    .await;

    // Update prices to process the deposit
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![vault::msg::PriceInfo {
                denom: "uatom".to_string(),
                price_usd: Decimal256::from_str("10.0").unwrap(),
                decimals: 0,
            }],
        },
    )
    .await;

    // Check that vault has assets and shares
    let total_shares = vault.querier.total_shares().await.unwrap();
    assert!(total_shares > Uint256::zero());

    let vault_assets = vault.querier.vault_assets().await.unwrap();
    assert_eq!(vault_assets.len(), 1);
    assert_eq!(vault_assets[0].denom, "uatom");
    assert_eq!(vault_assets[0].amount, Uint256::from(100_000_000u64));
}

#[tokio::test]
async fn test_vault_multi_denom_deposit() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));

    // Test multi-denom deposit with shared test logic
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user1.into(),
            deposit_amount: vec![
                coin(50_000_000, "uatom"),
                coin(30_000_000, "uosmo"),
                coin(20_000_000, "ujuno"),
            ],
        },
    )
    .await;

    // Update prices for all denominations
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("10.0").unwrap(),
                    decimals: 0,
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.0").unwrap(),
                    decimals: 0,
                },
                vault::msg::PriceInfo {
                    denom: "ujuno".to_string(),
                    price_usd: Decimal256::from_str("5.0").unwrap(),
                    decimals: 0,
                },
            ],
        },
    )
    .await;

    // Verify total vault value: (50M * 10) + (30M * 2) + (20M * 5) = 500M + 60M + 100M = 660M
    let vault_value = vault.querier.vault_value().await.unwrap();
    assert_eq!(vault_value, Decimal256::from_str("660000000").unwrap());
}

#[tokio::test]
async fn test_vault_withdrawal() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));

    // Make a deposit
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user1.clone().into(),
            deposit_amount: coins(100_000_000, "uatom"),
        },
    )
    .await;

    // Update prices to process the deposit
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![vault::msg::PriceInfo {
                denom: "uatom".to_string(),
                price_usd: Decimal256::from_str("10.0").unwrap(),
                decimals: 0,
            }],
        },
    )
    .await;

    // Get total shares and withdraw half
    let total_shares = vault.querier.total_shares().await.unwrap();
    let withdraw_shares = total_shares / Uint256::from(2u64);

    // Test withdrawal with shared test logic
    shared_test_withdrawal(
        &vault.querier,
        &vault.executor,
        VaultWithdrawalProps {
            user_addr: user1.into(),
            shares: withdraw_shares,
            expected_minimum_withdrawal: Uint256::from(40_000_000u64), // Approximate half of deposit value
        },
    )
    .await;
}

#[tokio::test]
async fn test_vault_whitelist_management() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());

    // Test adding new token to whitelist
    shared_test_whitelist_management(
        &vault.querier,
        &vault.executor,
        VaultWhitelistProps {
            to_add: Some(vec!["uwasm".to_string()]),
            to_remove: None,
            expected_final_denoms: vec![
                "uatom".to_string(),
                "uosmo".to_string(),
                "ujuno".to_string(),
                "uwasm".to_string(),
            ],
        },
    )
    .await;

    // Test removing token from whitelist
    shared_test_whitelist_management(
        &vault.querier,
        &vault.executor,
        VaultWhitelistProps {
            to_add: None,
            to_remove: Some(vec!["uwasm".to_string()]),
            expected_final_denoms: vec![
                "uatom".to_string(),
                "uosmo".to_string(),
                "ujuno".to_string(),
            ],
        },
    )
    .await;
}

#[tokio::test]
async fn test_vault_price_updates() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());

    // Test updating prices for multiple tokens
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("15.5").unwrap(),
                    decimals: 0,
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.3").unwrap(),
                    decimals: 0,
                },
                vault::msg::PriceInfo {
                    denom: "ujuno".to_string(),
                    price_usd: Decimal256::from_str("7.8").unwrap(),
                    decimals: 0,
                },
            ],
        },
    )
    .await;

    // Verify prices were set correctly
    let vault_state = vault.querier.vault_state().await.unwrap();
    assert_eq!(vault_state.prices.len(), 3);

    for price_info in &vault_state.prices {
        let queried_price = vault.querier.price(price_info.denom.clone()).await.unwrap();
        assert_eq!(queried_price.price_usd, price_info.price_usd);
        assert_eq!(queried_price.decimals, price_info.decimals);
    }
}

#[tokio::test]
async fn test_vault_error_handling() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));

    // Test various error conditions with shared test logic
    shared_test_error_handling(
        &vault.executor,
        VaultErrorHandlingProps {
            user_addr: user1.into(),
            invalid_denom: "nonwhitelisted".to_string(),
            invalid_amount: Uint256::from(1_000_000u64),
        },
    )
    .await;
}

#[tokio::test]
async fn test_vault_deposit_non_whitelisted_denom() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));

    // Test deposit with non-whitelisted token
    let result = vault
        .deposit(&user1.into(), &coins(100_000, "nonwhitelisted"))
        .await;

    // Should fail
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Cannot Sub")
            || error_msg.contains("Overflow")
            || error_msg.contains("Token not whitelisted")
    );
}

#[tokio::test]
async fn test_vault_comprehensive_workflow() {
    tracing_init();

    let app_client = AppClient::new("admin");
    let vault = VaultClient::new(app_client.clone());
    let user1 = app_client.with_app(|app| app.api().addr_make("user1"));
    let user2 = app_client.with_app(|app| app.api().addr_make("user2"));

    // Test comprehensive workflow with shared test logic
    shared_test_comprehensive_workflow(
        &vault.querier,
        &vault.executor,
        VaultComprehensiveWorkflowProps {
            user1: user1.into(),
            user2: user2.into(),
            user1_deposit: coins(100_000_000, "uatom"),
            user2_deposit: coins(200_000_000, "uosmo"),
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("15.0").unwrap(),
                    decimals: 0,
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.0").unwrap(),
                    decimals: 0,
                },
            ],
            new_whitelist_denom: "uwasm".to_string(),
        },
    )
    .await;
}
