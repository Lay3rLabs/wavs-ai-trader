use on_chain_tests::client::{vault::VaultClient, AppClient};
use ai_portfolio_utils::tracing::tracing_init;
use ai_portfolio_test_common::shared_tests::vault::{
    test_vault_instantiation as shared_test_instantiation,
    test_vault_deposit as shared_test_deposit,
    test_vault_whitelist_management as shared_test_whitelist_management,
    test_vault_price_updates as shared_test_price_updates,
    test_vault_withdrawal as shared_test_withdrawal,
    test_vault_error_handling as shared_test_error_handling,
    test_vault_comprehensive_workflow as shared_test_comprehensive_workflow,
    test_vault_multiple_deposits as shared_test_multiple_deposits,
    get_admin,
    VaultInstantiationProps, VaultDepositProps, VaultWhitelistProps,
    VaultPriceUpdateProps, VaultWithdrawalProps, VaultErrorHandlingProps,
    VaultComprehensiveWorkflowProps, VaultMultiDepositProps
};
use cosmwasm_std::{coin, coins, Uint256, Decimal256};
use std::str::FromStr;

#[tokio::test]
async fn test_vault_initialization() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;

    // Check that ownership is set correctly
    get_admin(&vault.querier, "owner_addr").await;

    // Test vault initialization with shared test logic
    shared_test_instantiation(
        &vault.querier,
        VaultInstantiationProps {
            service_manager: "service_manager".to_string(),
            initial_whitelisted_denoms: vec![
                "uatom".to_string(),
                "uosmo".to_string(),
            ],
            skip_entry_point: "skip_entry".to_string(),
        },
    ).await;
}

#[tokio::test]
async fn test_vault_single_deposit() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;
    let user_addr = app_client.rand_address().await;

    // Test single deposit with shared test logic
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user_addr.into(),
            deposit_amount: coins(1_000_000, "uatom"),
        },
    ).await;

    // Update prices to process the deposit
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("10.5").unwrap(),
                },
            ],
        },
    ).await;

    // Check that vault has assets and shares
    let total_shares = vault.querier.total_shares().await.unwrap();
    assert!(total_shares > Uint256::zero());

    let vault_assets = vault.querier.vault_assets().await.unwrap();
    assert_eq!(vault_assets.len(), 1);
    assert_eq!(vault_assets[0].denom, "uatom");
    assert_eq!(vault_assets[0].amount, Uint256::from(1_000_000u64));
}

#[tokio::test]
async fn test_vault_multi_denom_deposit() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;
    let user_addr = app_client.rand_address().await;

    // Test multi-denom deposit with shared test logic
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user_addr.into(),
            deposit_amount: vec![
                coin(500_000, "uatom"),
                coin(300_000, "uosmo"),
                coin(200_000, "ujuno"),
            ],
        },
    ).await;

    // Update prices for all denominations
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("10.5").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("1.2").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "ujuno".to_string(),
                    price_usd: Decimal256::from_str("5.0").unwrap(),
                },
            ],
        },
    ).await;

    // Verify total vault value
    let vault_value = vault.querier.vault_value().await.unwrap();
    assert!(vault_value > Decimal256::zero());
}

#[tokio::test]
async fn test_vault_withdrawal() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;
    let user_addr = app_client.rand_address().await;

    // Make a deposit
    shared_test_deposit(
        &vault.querier,
        &vault.executor,
        VaultDepositProps {
            user_addr: user_addr.clone().into(),
            deposit_amount: coins(1_000_000, "uatom"),
        },
    ).await;

    // Update prices to process the deposit
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("10.0").unwrap(),
                },
            ],
        },
    ).await;

    // Get total shares and withdraw half
    let total_shares = vault.querier.total_shares().await.unwrap();
    let withdraw_shares = total_shares / Uint256::from(2u64);

    // Test withdrawal with shared test logic
    shared_test_withdrawal(
        &vault.querier,
        &vault.executor,
        VaultWithdrawalProps {
            user_addr: user_addr.into(),
            shares: withdraw_shares,
            expected_minimum_withdrawal: Uint256::from(400_000u64), // Approximate half
        },
    ).await;
}

#[tokio::test]
async fn test_vault_whitelist_management() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;

    // Test adding new tokens to whitelist
    shared_test_whitelist_management(
        &vault.querier,
        &vault.executor,
        VaultWhitelistProps {
            to_add: Some(vec!["ujuno".to_string(), "uluna".to_string()]),
            to_remove: Some(vec!["uosmo".to_string()]),
            expected_final_denoms: vec![
                "uatom".to_string(),
                "ujuno".to_string(),
                "uluna".to_string(),
            ],
        },
    ).await;
}

#[tokio::test]
async fn test_vault_price_updates() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;

    // Test updating prices for multiple tokens
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("15.5").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.3").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "ujuno".to_string(),
                    price_usd: Decimal256::from_str("7.8").unwrap(),
                },
            ],
        },
    ).await;

    // Verify prices were set correctly
    let vault_state = vault.querier.vault_state().await.unwrap();
    assert_eq!(vault_state.prices.len(), 3);

    for price_info in &vault_state.prices {
        let queried_price = vault.querier.price(price_info.denom.clone()).await.unwrap();
        assert_eq!(queried_price, price_info.price_usd);
    }
}

#[tokio::test]
async fn test_vault_multiple_deposits() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;

    let user1 = app_client.rand_address().await;
    let user2 = app_client.rand_address().await;

    // Test multiple deposits with shared test logic
    shared_test_multiple_deposits(
        &vault.querier,
        &vault.executor,
        VaultMultiDepositProps {
            deposits: vec![
                (user1.into(), vec![cosmwasm_std::coin(1_000_000, "uatom")]),
                (user2.into(), vec![cosmwasm_std::coin(500_000, "uosmo")]),
            ],
        },
    ).await;

    // Update prices to process deposits
    shared_test_price_updates(
        &vault.querier,
        &vault.executor,
        VaultPriceUpdateProps {
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("10.0").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.0").unwrap(),
                },
            ],
        },
    ).await;

    // Verify vault has assets and shares
    let total_shares = vault.querier.total_shares().await.unwrap();
    assert!(total_shares > Uint256::zero());

    let vault_assets = vault.querier.vault_assets().await.unwrap();
    assert_eq!(vault_assets.len(), 2);
}

#[tokio::test]
async fn test_vault_error_handling() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;
    let user_addr = app_client.rand_address().await;

    // Test various error conditions with shared test logic
    shared_test_error_handling(
        &vault.executor,
        VaultErrorHandlingProps {
            user_addr: user_addr.into(),
            invalid_denom: "nonwhitelisted".to_string(),
            invalid_amount: Uint256::from(1_000_000u64),
        },
    ).await;
}

#[tokio::test]
async fn test_vault_comprehensive_workflow() {
    tracing_init();

    let app_client = AppClient::new().await;
    let vault = VaultClient::new(app_client.clone(), None).await;

    let user1 = app_client.rand_address().await;
    let user2 = app_client.rand_address().await;

    // Test comprehensive workflow with shared test logic
    shared_test_comprehensive_workflow(
        &vault.querier,
        &vault.executor,
        VaultComprehensiveWorkflowProps {
            user1: user1.into(),
            user2: user2.into(),
            user1_deposit: coins(1_000_000, "uatom"),
            user2_deposit: coins(500_000, "uosmo"),
            prices: vec![
                vault::msg::PriceInfo {
                    denom: "uatom".to_string(),
                    price_usd: Decimal256::from_str("15.0").unwrap(),
                },
                vault::msg::PriceInfo {
                    denom: "uosmo".to_string(),
                    price_usd: Decimal256::from_str("2.0").unwrap(),
                },
            ],
            new_whitelist_denom: "uwasm".to_string(),
        },
    ).await;
}