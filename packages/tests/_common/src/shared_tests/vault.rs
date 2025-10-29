use ai_portfolio_utils::{
    addr::AnyAddr,
    client::vault::{VaultExecutor, VaultQuerier},
};

pub async fn get_admin(querier: &VaultQuerier, expected: &str) {
    let ownership = querier.ownership().await.unwrap();
    assert_eq!(ownership.owner.unwrap().to_string(), expected);
}

pub struct VaultInstantiationProps {
    pub service_manager: String,
    pub initial_whitelisted_denoms: Vec<String>,
    pub skip_entry_point: String,
}

pub async fn test_vault_instantiation(querier: &VaultQuerier, props: VaultInstantiationProps) {
    let VaultInstantiationProps {
        service_manager: _,
        initial_whitelisted_denoms,
        skip_entry_point: _,
    } = props;

    // Verify initial state
    let total_shares = querier.total_shares().await.unwrap();
    assert_eq!(total_shares, cosmwasm_std::Uint256::zero());

    let vault_value = querier.vault_value().await.unwrap();
    assert_eq!(vault_value, cosmwasm_std::Decimal256::zero());

    let whitelisted_denoms = querier.whitelisted_denoms().await.unwrap();
    assert_eq!(whitelisted_denoms.len(), initial_whitelisted_denoms.len());

    for denom in &initial_whitelisted_denoms {
        assert!(whitelisted_denoms.contains(denom));
    }

    let vault_assets = querier.vault_assets().await.unwrap();
    assert_eq!(vault_assets.len(), 0);

    let vault_state = querier.vault_state().await.unwrap();
    assert_eq!(vault_state.funds.len(), 0);
    assert_eq!(vault_state.pending_assets.len(), 0);
    assert_eq!(vault_state.prices.len(), 0);
    assert_eq!(vault_state.tvl, cosmwasm_std::Decimal256::zero());
}

pub struct VaultDepositProps {
    pub user_addr: AnyAddr,
    pub deposit_amount: Vec<cosmwasm_std::Coin>,
}

pub async fn test_vault_deposit(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultDepositProps,
) {
    let VaultDepositProps {
        user_addr,
        deposit_amount,
    } = props;

    // Execute deposit
    executor.deposit(&user_addr, &deposit_amount).await.unwrap();

    // Check deposit request was created
    let deposit_request = querier.deposit_request(1).await.unwrap();
    assert_eq!(deposit_request.id, 1);
    assert_eq!(deposit_request.user.to_string(), user_addr.to_string());
    assert_eq!(deposit_request.coins, deposit_amount);
    assert!(matches!(
        deposit_request.state,
        vault::msg::DepositState::Pending
    ));
}

pub struct VaultWhitelistProps {
    pub to_add: Option<Vec<String>>,
    pub to_remove: Option<Vec<String>>,
    pub expected_final_denoms: Vec<String>,
}

pub async fn test_vault_whitelist_management(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultWhitelistProps,
) {
    let VaultWhitelistProps {
        to_add,
        to_remove,
        expected_final_denoms,
    } = props;

    // Update whitelist
    executor
        .update_whitelist(to_add.clone(), to_remove.clone())
        .await
        .unwrap();

    // Verify whitelist was updated correctly
    let whitelisted = querier.whitelisted_denoms().await.unwrap();
    assert_eq!(whitelisted.len(), expected_final_denoms.len());

    for denom in &expected_final_denoms {
        assert!(whitelisted.contains(denom));
    }
}

pub struct VaultPriceUpdateProps {
    pub prices: Vec<vault::msg::PriceInfo>,
}

pub async fn test_vault_price_updates(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultPriceUpdateProps,
) {
    let VaultPriceUpdateProps { prices } = props;

    // Update prices
    executor.update_prices(prices.clone(), None).await.unwrap();

    // Verify prices were updated
    let vault_state = querier.vault_state().await.unwrap();
    assert_eq!(vault_state.prices.len(), prices.len());

    for price_info in &prices {
        let stored_price = vault_state
            .prices
            .iter()
            .find(|p| p.denom == price_info.denom)
            .unwrap();
        assert_eq!(stored_price.price_usd, price_info.price_usd);
    }
}

pub struct VaultWithdrawalProps {
    pub user_addr: AnyAddr,
    pub shares: cosmwasm_std::Uint256,
    pub expected_minimum_withdrawal: cosmwasm_std::Uint256,
}

pub async fn test_vault_withdrawal(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultWithdrawalProps,
) {
    let VaultWithdrawalProps {
        user_addr,
        shares,
        expected_minimum_withdrawal: _,
    } = props;

    // Get state before withdrawal
    let total_shares_before = querier.total_shares().await.unwrap();

    // Execute withdrawal
    executor.withdraw(&user_addr, shares).await.unwrap();

    // Verify shares were burned
    let total_shares_after = querier.total_shares().await.unwrap();
    assert_eq!(total_shares_after, total_shares_before - shares);
}

pub struct VaultErrorHandlingProps {
    pub user_addr: AnyAddr,
    pub invalid_denom: String,
    pub invalid_amount: cosmwasm_std::Uint256,
}

pub async fn test_vault_error_handling(executor: &VaultExecutor, props: VaultErrorHandlingProps) {
    let VaultErrorHandlingProps {
        user_addr,
        invalid_denom,
        invalid_amount,
    } = props;

    // Test deposit with non-whitelisted token
    let invalid_deposit = vec![cosmwasm_std::Coin {
        denom: invalid_denom,
        amount: 100000u128.into(),
    }];

    let result = executor.deposit(&user_addr, &invalid_deposit).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Cannot Sub")
            || error_msg.contains("Overflow")
            || error_msg.contains("Token not whitelisted")
    );

    // Test withdraw zero shares
    let result = executor
        .withdraw(&user_addr, cosmwasm_std::Uint256::zero())
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot withdraw zero shares"));

    // Test withdraw insufficient shares
    let result = executor.withdraw(&user_addr, invalid_amount).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Insufficient shares"));
}

pub struct VaultComprehensiveWorkflowProps {
    pub user1: AnyAddr,
    pub user2: AnyAddr,
    pub user1_deposit: Vec<cosmwasm_std::Coin>,
    pub user2_deposit: Vec<cosmwasm_std::Coin>,
    pub prices: Vec<vault::msg::PriceInfo>,
    pub new_whitelist_denom: String,
}

pub async fn test_vault_comprehensive_workflow(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultComprehensiveWorkflowProps,
) {
    let VaultComprehensiveWorkflowProps {
        user1,
        user2,
        user1_deposit,
        user2_deposit,
        prices,
        new_whitelist_denom,
    } = props;

    // User1 makes first deposit
    executor.deposit(&user1, &user1_deposit).await.unwrap();

    // User2 makes second deposit
    executor.deposit(&user2, &user2_deposit).await.unwrap();

    // Verify both deposit requests exist
    let deposit1 = querier.deposit_request(1).await.unwrap();
    let deposit2 = querier.deposit_request(2).await.unwrap();
    assert_eq!(deposit1.user.to_string(), user1.to_string());
    assert_eq!(deposit2.user.to_string(), user2.to_string());
    assert_eq!(deposit1.coins, user1_deposit);
    assert_eq!(deposit2.coins, user2_deposit);

    // Update prices to process deposits
    executor.update_prices(prices, None).await.unwrap();

    // Verify deposits were processed (check for completed state)
    let deposit1_processed = querier.deposit_request(1).await.unwrap();
    let deposit2_processed = querier.deposit_request(2).await.unwrap();
    assert!(matches!(
        deposit1_processed.state,
        vault::msg::DepositState::Completed { .. }
    ));
    assert!(matches!(
        deposit2_processed.state,
        vault::msg::DepositState::Completed { .. }
    ));

    // Verify vault has assets and shares
    let total_shares = querier.total_shares().await.unwrap();
    assert!(total_shares > cosmwasm_std::Uint256::zero());

    let vault_assets = querier.vault_assets().await.unwrap();
    assert!(!vault_assets.is_empty());

    // Add new token to whitelist
    executor
        .update_whitelist(Some(vec![new_whitelist_denom.clone()]), None)
        .await
        .unwrap();

    // Verify whitelist was updated
    let whitelisted = querier.whitelisted_denoms().await.unwrap();
    assert!(whitelisted.contains(&new_whitelist_denom));
}

pub struct VaultMultiDepositProps {
    pub deposits: Vec<(AnyAddr, Vec<cosmwasm_std::Coin>)>,
}

pub async fn test_vault_multiple_deposits(
    querier: &VaultQuerier,
    executor: &VaultExecutor,
    props: VaultMultiDepositProps,
) {
    let VaultMultiDepositProps { deposits } = props;

    let mut expected_vault_assets: std::collections::HashMap<String, cosmwasm_std::Uint256> =
        std::collections::HashMap::new();

    for (i, (user_addr, deposit_amount)) in deposits.into_iter().enumerate() {
        // Execute deposit
        executor.deposit(&user_addr, &deposit_amount).await.unwrap();

        // Track expected vault assets
        for coin in &deposit_amount {
            *expected_vault_assets
                .entry(coin.denom.clone())
                .or_insert_with(cosmwasm_std::Uint256::zero) += coin.amount;
        }

        // Check deposit request was created
        let deposit_request = querier.deposit_request((i + 1) as u64).await.unwrap();
        assert_eq!(deposit_request.id, (i + 1) as u64);
        assert_eq!(deposit_request.user.to_string(), user_addr.to_string());
        assert_eq!(deposit_request.coins, deposit_amount);
        assert!(matches!(
            deposit_request.state,
            vault::msg::DepositState::Pending
        ));
    }

    // Verify final vault assets
    let vault_assets = querier.vault_assets().await.unwrap();
    assert_eq!(vault_assets.len(), expected_vault_assets.len());

    for vault_coin in &vault_assets {
        let expected_amount = expected_vault_assets.get(&vault_coin.denom).unwrap();
        assert_eq!(vault_coin.amount, *expected_amount);
    }
}
