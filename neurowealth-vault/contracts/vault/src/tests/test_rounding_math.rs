//! Tests for rounding and share math properties
//!
//! These tests ensure the vault's share-based accounting maintains mathematical invariants:
//! - Deposit then withdraw never returns more than total assets
//! - Shares/asset conversions are monotonic and consistent across sequences
//! - Zero/near-zero rounding edges are handled correctly
//! - Multi-user sequences maintain fairness

use super::utils::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Test that deposit then withdraw never returns more than total assets
#[test]
fn test_deposit_withdraw_never_exceeds_total_assets() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128; // 10 USDC

    // Deposit
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // Add some yield to make it interesting
    let yield_amount = 2_000_000_i128; // 2 USDC yield
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &((deposit_amount   + yield_amount)), &false, &0);

    let total_assets_before = client.get_total_assets();
    let vault_balance_before = token_client.balance(&contract_id);

    // Withdraw everything
    let withdrawn_amount = client.withdraw_all(&user);

    let total_assets_after = client.get_total_assets();
    let vault_balance_after = token_client.balance(&contract_id);

    // Invariant: Total assets should never increase from a withdrawal
    assert!(
        total_assets_after <= total_assets_before,
        "Total assets should not increase after withdrawal"
    );

    // Invariant: Vault balance should never increase from a withdrawal
    assert!(
        vault_balance_after <= vault_balance_before,
        "Vault balance should not increase after withdrawal"
    );

    // Invariant: Withdrawn amount should not exceed user's proportional share
    assert!(
        withdrawn_amount <= deposit_amount + yield_amount,
        "Withdrawn amount should not exceed total contribution plus yield"
    );

    // Edge case: Withdrawal should never be negative
    assert!(
        withdrawn_amount >= 0,
        "Withdrawn amount should never be negative"
    );
}

/// Test multi-user sequences maintain fairness and invariants
#[test]
fn test_multi_user_deposit_withdraw_invariants() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    // Create multiple users with different deposit amounts
    let users = [
        (Address::generate(&env), 5_000_000_i128),  // 5 USDC
        (Address::generate(&env), 10_000_000_i128), // 10 USDC
        (Address::generate(&env), 15_000_000_i128), // 15 USDC
        (Address::generate(&env), 20_000_000_i128), // 20 USDC
    ];

    let mut total_deposited = 0_i128;

    // All users deposit
    for (user, amount) in users.iter() {
        mint_and_deposit(&env, &client, &usdc_token, user, *amount);
        total_deposited += amount;
    }

    // Add yield to the system
    let yield_amount = 8_000_000_i128; // 8 USDC yield
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &((total_deposited   + yield_amount)), &false, &0);

    let total_assets_before = client.get_total_assets();
    let _total_shares_before = client.get_total_shares();

    // Users withdraw in random order
    let mut total_withdrawn = 0_i128;
    for (user, original_deposit) in users.iter() {
        let _user_shares_before = client.get_shares(user);
        let _user_balance_before = client.get_balance(user);

        let withdrawn_amount = client.withdraw_all(user);
        total_withdrawn += withdrawn_amount;

        // Invariant: Withdrawn amount should be reasonable
        assert!(
            withdrawn_amount >= 0,
            "Withdrawn amount should never be negative"
        );
        assert!(
            withdrawn_amount <= total_assets_before,
            "Single withdrawal cannot exceed total assets"
        );

        // Invariant: User should get at least their principal (unless rounding)
        // In extreme cases, rounding might cause tiny losses, but never gains
        assert!(
            withdrawn_amount <= original_deposit + yield_amount,
            "User cannot withdraw more than their contribution plus proportional yield"
        );

        // Verify user state is properly reset
        assert_eq!(
            client.get_shares(user),
            0,
            "User shares should be zero after full withdrawal"
        );
        assert_eq!(
            client.get_balance(user),
            0,
            "User balance should be zero after full withdrawal"
        );
    }

    let total_assets_after = client.get_total_assets();
    let total_shares_after = client.get_total_shares();

    // Invariant: All shares should be burned
    assert_eq!(
        total_shares_after, 0,
        "Total shares should be zero after all withdrawals"
    );

    // Invariant: Total assets should be reasonable (may have tiny rounding remainder)
    assert!(
        total_assets_after >= 0,
        "Total assets should never be negative"
    );
    assert!(
        total_assets_after <= total_deposited + yield_amount,
        "Total assets should not exceed total deposits plus yield"
    );

    // Invariant: Total withdrawn should not exceed total assets
    assert!(
        total_withdrawn <= total_assets_before,
        "Total withdrawn cannot exceed initial total assets"
    );
}

/// Test shares/asset conversions are monotonic and consistent
#[test]
fn test_shares_asset_conversions_monotonic() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let initial_deposit = 10_000_000_i128;

    // Initial state: empty vault
    assert_eq!(
        client.get_total_shares(),
        0,
        "Initial shares should be zero"
    );
    assert_eq!(
        client.get_total_assets(),
        0,
        "Initial assets should be zero"
    );

    // Test bootstrap case (first deposit)
    let shares_for_10 = client.preview_deposit_to_shares(&initial_deposit);
    assert_eq!(
        shares_for_10, initial_deposit,
        "First deposit should be 1:1 shares"
    );

    mint_and_deposit(&env, &client, &usdc_token, &user, initial_deposit);

    // Add yield to increase share price
    let yield_amount = 5_000_000_i128;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &((initial_deposit   + yield_amount)), &false, &0);

    let total_assets_after_yield = client.get_total_assets();
    let total_shares_after_yield = client.get_total_shares();

    // Share price should have increased
    assert!(
        total_assets_after_yield > total_shares_after_yield,
        "Share price should increase after yield"
    );

    // Test conversion monotonicity using sufficiently separated inputs to avoid rounding ties.
    let shares_for_20 = client.preview_deposit_to_shares(&20_000_000_i128);
    let shares_for_60 = client.preview_deposit_to_shares(&60_000_000_i128);

    assert!(
        shares_for_60 > shares_for_20,
        "More assets should give more shares"
    );

    // Test conversion consistency: shares -> assets -> shares should be consistent
    let assets_from_shares = client.preview_shares_to_assets(&shares_for_10);
    let shares_back = client.preview_deposit_to_shares(&assets_from_shares);

    // Due to rounding, we might lose precision but never gain
    assert!(
        shares_back <= shares_for_10,
        "Round-trip conversion should not gain shares"
    );

    // Test that share price is consistent
    let share_price_1 = total_assets_after_yield * 1_000_000 / total_shares_after_yield;
    let assets_from_1m_shares = client.preview_shares_to_assets(&1_000_000_i128);
    let calculated_price = assets_from_1m_shares;

    assert_eq!(
        share_price_1, calculated_price,
        "Share price should be consistent across calculations"
    );
}

/// Test zero and near-zero rounding edge cases
#[test]
fn test_zero_near_zero_rounding_edges() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);

    // Test zero amount conversions
    let shares_for_zero = client.preview_deposit_to_shares(&0);
    let assets_for_zero = client.preview_shares_to_assets(&0);

    assert_eq!(shares_for_zero, 0, "Zero assets should give zero shares");
    assert_eq!(assets_for_zero, 0, "Zero shares should give zero assets");

    // Test sub-minimum amount conversions (1 unit = 0.0000001 USDC).
    // This should be safe for previews, but deposits will be rejected by the min-deposit guard.
    let sub_min_amount = 1_i128;
    let _shares_for_sub_min = client.preview_deposit_to_shares(&sub_min_amount);

    // Deposit the configured minimum (1 USDC = 1_000_000 with 7 decimals).
    let min_deposit = 1_000_000_i128;
    token_client.mint(&user, &min_deposit);
    client.deposit(&user, &min_deposit);

    // Test that tiny amounts don't break the system
    let tiny_yield = 1_i128;
    token_client.mint(&contract_id, &tiny_yield);
    client.update_total_assets(&agent, &((min_deposit   + tiny_yield)), &false, &0);

    // Withdraw should work even with tiny amounts
    let withdrawn = client.withdraw_all(&user);
    assert!(withdrawn >= 0, "Withdrawal should work with tiny amounts");
    assert!(
        withdrawn <= min_deposit + tiny_yield,
        "Withdrawal should not exceed total"
    );

    // Test rounding with very small amounts in large pool
    let big_user = Address::generate(&env);
    let big_deposit = 10_000_000_000_i128; // 1,000 USDC (default max deposit)
    mint_and_deposit(&env, &client, &usdc_token, &big_user, big_deposit);

    let small_user = Address::generate(&env);
    let small_deposit = 1_000_000_i128; // 1 USDC (minimum deposit)
    token_client.mint(&small_user, &small_deposit);
    client.deposit(&small_user, &small_deposit);

    // Both users should be able to withdraw
    let big_withdrawn = client.withdraw_all(&big_user);
    let small_withdrawn = client.withdraw_all(&small_user);

    assert!(big_withdrawn > 0, "Big user should withdraw something");
    assert!(
        small_withdrawn >= 0,
        "Small user should not lose money due to rounding"
    );
    assert!(
        big_withdrawn + small_withdrawn <= big_deposit + small_deposit,
        "Total withdrawn should not exceed total deposited"
    );
}

/// Test that share price calculations are consistent
#[test]
fn test_share_price_consistency() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Multiple deposits at different times
    let deposit1 = 10_000_000_i128;
    let deposit2 = 5_000_000_i128;

    mint_and_deposit(&env, &client, &usdc_token, &user1, deposit1);

    // Add yield between deposits
    let yield1 = 2_000_000_i128;
    token_client.mint(&contract_id, &yield1);
    client.update_total_assets(&agent, &((deposit1   + yield1)), &false, &0);

    let shares1_after_yield = client.get_shares(&user1);
    let assets1_from_shares = client.preview_shares_to_assets(&shares1_after_yield);

    // Second user deposits at higher share price
    mint_and_deposit(&env, &client, &usdc_token, &user2, deposit2);

    let shares2 = client.get_shares(&user2);
    let assets2_from_shares = client.preview_shares_to_assets(&shares2);

    // Both users should get proportional assets
    assert!(
        assets1_from_shares > deposit1,
        "User1 should have earned yield"
    );
    let diff2 = (assets2_from_shares - deposit2).abs();
    assert!(
        diff2 <= 1,
        "User2 should get approximately their deposit (allowing 1 unit rounding)"
    );

    // Add more yield
    let yield2 = 3_000_000_i128;
    token_client.mint(&contract_id, &yield2);
    client.update_total_assets(&agent, &((((deposit1   + deposit2   + yield1     + yield2)))), &false, &0);

    // Check that share price is applied consistently
    let final_assets1 = client.preview_shares_to_assets(&shares1_after_yield);
    let final_assets2 = client.preview_shares_to_assets(&shares2);

    // Both should have benefited from second yield proportionally
    assert!(
        final_assets1 > assets1_from_shares,
        "User1 should benefit from second yield"
    );
    assert!(
        final_assets2 > assets2_from_shares,
        "User2 should benefit from second yield"
    );

    // Verify total assets consistency
    let total_assets = client.get_total_assets();
    let calculated_total = final_assets1 + final_assets2;

    // Should be very close (allowing for tiny rounding differences)
    let diff = (total_assets - calculated_total).abs();
    assert!(
        diff <= 1,
        "Total assets should match sum of user assets (allowing 1 unit rounding)"
    );
}

/// Test extreme rounding scenarios
#[test]
fn test_extreme_rounding_scenarios() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    // Use minimum deposit to avoid triggering the min-deposit guard.
    let tiny_deposit = 1_000_000_i128;

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    // Each user deposits tiny amount
    token_client.mint(&user1, &tiny_deposit);
    client.deposit(&user1, &tiny_deposit);

    token_client.mint(&user2, &tiny_deposit);
    client.deposit(&user2, &tiny_deposit);

    token_client.mint(&user3, &tiny_deposit);
    client.deposit(&user3, &tiny_deposit);

    let total_deposited = tiny_deposit * 3;
    assert_eq!(client.get_total_shares(), total_deposited);
    assert_eq!(client.get_total_assets(), total_deposited);

    // Add tiny yield to create rounding challenges
    let tiny_yield = 1_i128;
    token_client.mint(&contract_id, &tiny_yield);
    client.update_total_assets(&agent, &((total_deposited   + tiny_yield)), &false, &0);

    // All users withdraw - should handle rounding gracefully
    let withdrawn1 = client.withdraw_all(&user1);
    let withdrawn2 = client.withdraw_all(&user2);
    let withdrawn3 = client.withdraw_all(&user3);
    let total_withdrawn = withdrawn1 + withdrawn2 + withdrawn3;

    // Check each withdrawal
    assert!(withdrawn1 >= 0, "No user should have negative withdrawal");
    assert!(
        withdrawn1 <= tiny_deposit + 1,
        "No user should gain from rounding"
    );
    assert!(withdrawn2 >= 0, "No user should have negative withdrawal");
    assert!(
        withdrawn2 <= tiny_deposit + 1,
        "No user should gain from rounding"
    );
    assert!(withdrawn3 >= 0, "No user should have negative withdrawal");
    assert!(
        withdrawn3 <= tiny_deposit + 1,
        "No user should gain from rounding"
    );

    // System should remain stable
    assert_eq!(client.get_total_shares(), 0, "All shares should be burned");
    assert!(
        client.get_total_assets() >= 0,
        "Total assets should not be negative"
    );
    assert!(
        total_withdrawn <= total_deposited + tiny_yield,
        "Total withdrawn should not exceed total plus yield"
    );
}

/// Test that preview functions match actual conversions
#[test]
fn test_preview_functions_match_actual_conversions() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 7_543_210_i128; // Odd amount to test rounding

    // Preview should match actual
    let previewed_shares = client.preview_deposit_to_shares(&deposit_amount);

    token_client.mint(&user, &deposit_amount);
    client.deposit(&user, &deposit_amount);

    let actual_shares = client.get_shares(&user);
    assert_eq!(
        previewed_shares, actual_shares,
        "Preview should match actual shares"
    );

    // Add yield
    let yield_amount = 2_345_678_i128;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &((deposit_amount   + yield_amount)), &false, &0);

    // Preview withdrawal should match actual
    let previewed_assets = client.preview_shares_to_assets(&actual_shares);
    let withdrawn_assets = client.withdraw_all(&user);

    // Should be very close (allowing for 1 unit rounding difference)
    let diff = (previewed_assets - withdrawn_assets).abs();
    assert!(
        diff <= 1,
        "Preview should match actual withdrawal (allowing 1 unit rounding)"
    );
}
