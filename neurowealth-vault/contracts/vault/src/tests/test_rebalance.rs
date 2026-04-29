//! Tests for rebalance functionality

use super::utils::*;
use crate::{BlendWithdrawEvent, RebalanceEvent};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, TryFromVal};

#[test]
fn test_agent_can_rebalance_with_custom_protocol() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Verify agent is set correctly
    assert_eq!(client.get_agent(), agent);

    // Use "none" protocol — always safe, no external pool required
    let protocol = symbol_short!("none");
    let expected_apy = 500_i128; // 5% APY in basis points

    // Should succeed with mock_all_auths (require_is_agent passes)
    client.rebalance(&protocol, &expected_apy);
}

#[test]
fn test_rebalance_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Set up Blend pool and deposit so there are assets
    client.set_blend_pool(&owner, &blend_pool);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    client.rebalance(&symbol_short!("blend"), &500_i128);

    let rebalance_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("rebalance"));
    assert!(
        !rebalance_events.is_empty(),
        "Rebalance should emit an event"
    );

    // Assert storage state change: CurrentProtocol should be "blend"
    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("blend"),
        "CurrentProtocol should be 'blend' after rebalance to blend"
    );
}

#[test]
fn test_rebalance_storage_current_protocol_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.set_blend_pool(&owner, &blend_pool);

    // Initial state: no protocol set
    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("none"),
        "Initial CurrentProtocol should be 'none'"
    );

    // Deposit so vault has funds to supply
    let user = Address::generate(&env);
    mint_and_deposit(&env, &client, &usdc_token, &user, 10_000_000_i128);

    // Rebalance to blend
    client.rebalance(&symbol_short!("blend"), &500_i128);

    // Assert storage state changed
    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("blend"),
        "CurrentProtocol should be 'blend' after rebalance"
    );
}

#[test]
fn test_rebalance_storage_current_protocol_changes_to_none() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.set_blend_pool(&owner, &blend_pool);

    let user = Address::generate(&env);
    mint_and_deposit(&env, &client, &usdc_token, &user, 10_000_000_i128);

    // First rebalance to blend
    client.rebalance(&symbol_short!("blend"), &500_i128);
    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("blend"),
        "CurrentProtocol should be 'blend'"
    );

    // Then rebalance to none
    client.rebalance(&symbol_short!("none"), &0_i128);

    // Assert storage state changed to none
    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("none"),
        "CurrentProtocol should be 'none' after rebalance to none"
    );
}

#[test]
fn test_set_blend_pool_storage() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Initially no blend pool
    assert!(
        client.get_blend_pool().is_none(),
        "BlendPool should be None before set_blend_pool"
    );

    // Set blend pool
    client.set_blend_pool(&owner, &blend_pool);

    // Assert storage state changed
    assert_eq!(
        client.get_blend_pool(),
        Some(blend_pool.clone()),
        "BlendPool should be set to the provided address"
    );
}

#[test]
fn test_rebalance_with_none_protocol_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // "none" protocol just sets current protocol to "none" — always safe to call
    client.rebalance(&symbol_short!("none"), &0_i128);
}

#[test]
fn test_rebalance_with_blend_after_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    client.set_blend_pool(&owner, &blend_pool);

    // Deposit so vault has a token balance to supply
    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    client.rebalance(&symbol_short!("blend"), &500_i128);

    let token_client = TestTokenClient::new(&env, &usdc_token);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool);

    assert_eq!(blend_client.supplied(&usdc_token), deposit_amount);
    assert_eq!(token_client.balance(&contract_id), 0);
    assert_eq!(token_client.balance(&blend_pool), deposit_amount);
    assert_eq!(token_client.allowance(&contract_id, &blend_pool), 0);
}

#[test]
fn test_rebalance_apy_parameter_accepted() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Various APY values should be accepted without panicking
    client.rebalance(&symbol_short!("none"), &0_i128);
    client.rebalance(&symbol_short!("none"), &850_i128);
    client.rebalance(&symbol_short!("none"), &2000_i128);
}

#[test]
#[should_panic(expected = "vault: paused")]
fn test_rebalance_while_paused_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Pause the vault
    client.pause(&owner);
    assert!(client.is_paused());

    client.rebalance(&symbol_short!("none"), &500_i128);
}

#[test]
#[should_panic(expected = "vault: blend pool not configured")]
fn test_blend_rebalance_without_pool_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Deposit so vault_balance > 0, triggering the blend pool check
    let user = Address::generate(&env);
    let deposit_amount = 5_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // blend pool not set → should panic
    client.rebalance(&symbol_short!("blend"), &500_i128);
}

#[test]
fn test_mock_token_transfer_from_uses_and_decrements_allowance() {
    let env = Env::default();
    env.mock_all_auths();

    let token = env.register_contract(None, TestToken);
    let token_client = TestTokenClient::new(&env, &token);

    let owner = Address::generate(&env);
    let spender = Address::generate(&env);
    let recipient = Address::generate(&env);

    token_client.mint(&owner, &10_000_000_i128);
    token_client.approve(&owner, &spender, &6_000_000_i128, &10_000_u32);

    assert_eq!(token_client.allowance(&owner, &spender), 6_000_000_i128);

    token_client.transfer_from(&spender, &owner, &recipient, &4_000_000_i128);

    assert_eq!(token_client.balance(&owner), 6_000_000_i128);
    assert_eq!(token_client.balance(&recipient), 4_000_000_i128);
    assert_eq!(token_client.allowance(&owner, &spender), 2_000_000_i128);
}

#[test]
#[should_panic(expected = "vault: unsupported protocol")]
fn test_rebalance_with_unsupported_protocol_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // "balanced" is not a supported protocol — should panic
    client.rebalance(&symbol_short!("balanced"), &500_i128);
}

#[test]
fn test_blend_supply_and_withdraw_with_events() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool);

    // Configure Blend pool
    client.set_blend_pool(&owner, &blend_pool);

    // Deposit funds into vault
    let user = Address::generate(&env);
    let deposit_amount = 20_000_000_i128; // 20 USDC
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // Verify initial state
    assert_eq!(client.get_total_assets(), deposit_amount);
    assert_eq!(token_client.balance(&contract_id), deposit_amount);
    assert_eq!(blend_client.supplied(&usdc_token), 0);

    // Rebalance to Blend (supply)
    client.rebalance(&symbol_short!("blend"), &850_i128);

    // Verify funds moved to Blend
    assert_eq!(
        token_client.balance(&contract_id),
        0,
        "Vault should have 0 USDC after supply"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        deposit_amount,
        "Blend should have all USDC"
    );
    assert_eq!(client.get_current_protocol(), symbol_short!("blend"));

    // Verify BlendSupplyEvent was emitted
    let supply_events = find_events_by_topic(env.events().all(), &env, symbol_short!("blend_sup"));
    assert!(
        !supply_events.is_empty(),
        "BlendSupplyEvent should be emitted"
    );

    // User withdraws half their balance
    let withdraw_amount = 10_000_000_i128; // 10 USDC
    client.withdraw(&user, &withdraw_amount);

    // Verify funds were pulled from Blend and given to user
    assert_eq!(
        token_client.balance(&user),
        withdraw_amount,
        "User should receive withdrawn USDC"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        deposit_amount - withdraw_amount,
        "Blend should have remaining USDC"
    );

    // Verify BlendWithdrawEvent was emitted
    let withdraw_events = find_events_by_topic(env.events().all(), &env, symbol_short!("blend_wd"));
    assert!(
        !withdraw_events.is_empty(),
        "BlendWithdrawEvent should be emitted"
    );

    // Rebalance back to none (withdraw all from Blend)
    client.rebalance(&symbol_short!("none"), &0_i128);

    // Verify all funds withdrawn from Blend
    assert_eq!(client.get_current_protocol(), symbol_short!("none"));
    assert_eq!(
        token_client.balance(&contract_id),
        deposit_amount - withdraw_amount,
        "Vault should have remaining USDC"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        0,
        "Blend should have 0 USDC"
    );

    // Verify second BlendWithdrawEvent was emitted
    let all_withdraw_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("blend_wd"));
    assert!(
        all_withdraw_events.len() >= 2,
        "Should have at least 2 BlendWithdrawEvents"
    );
}

#[test]
fn test_rebalance_blend_to_none_withdraws_all_and_updates_state_and_events() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool);

    client.set_blend_pool(&owner, &blend_pool);

    let user = Address::generate(&env);
    let deposit_amount = 30_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    client.rebalance(&symbol_short!("blend"), &900_i128);

    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("blend"),
        "CurrentProtocol should be 'blend' after rebalance to blend"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        deposit_amount,
        "All deposited funds should be supplied to Blend"
    );
    assert_eq!(
        token_client.balance(&contract_id),
        0,
        "Vault should hold no idle USDC while fully allocated to Blend"
    );

    let none_apy = 0_i128;
    client.rebalance(&symbol_short!("none"), &none_apy);

    assert_eq!(
        client.get_current_protocol(),
        symbol_short!("none"),
        "CurrentProtocol should be 'none' after rebalance to none"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        0,
        "Blend should be fully withdrawn after rebalance to none"
    );
    assert_eq!(
        token_client.balance(&contract_id),
        deposit_amount,
        "All funds should be pulled back to vault after rebalance to none"
    );

    let blend_withdraw_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("blend_wd"));
    assert!(
        !blend_withdraw_events.is_empty(),
        "BlendWithdrawEvent should be emitted when switching blend -> none"
    );

    let (_, _, blend_withdraw_data) = blend_withdraw_events
        .last()
        .expect("Expected at least one blend_wd event");
    let blend_withdraw_event = BlendWithdrawEvent::try_from_val(&env, blend_withdraw_data)
        .expect("blend_wd data should decode to BlendWithdrawEvent");

    assert_eq!(
        blend_withdraw_event.requested_amount, deposit_amount,
        "blend_wd requested_amount should match full deployed balance"
    );
    assert_eq!(
        blend_withdraw_event.amount_received, deposit_amount,
        "blend_wd amount_received should match full withdrawal"
    );
    assert!(
        blend_withdraw_event.success,
        "blend_wd event should mark withdrawal as successful"
    );

    let rebalance_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("rebalance"));
    let (_, _, rebalance_data) = rebalance_events
        .last()
        .expect("Expected rebalance event for none transition");
    let rebalance_event = RebalanceEvent::try_from_val(&env, rebalance_data)
        .expect("rebalance data should decode to RebalanceEvent");

    assert_eq!(
        rebalance_event.protocol,
        symbol_short!("none"),
        "rebalance event protocol should reflect target none"
    );
    assert_eq!(
        rebalance_event.expected_apy, none_apy,
        "rebalance event APY should match provided value"
    );
}

#[test]
#[should_panic(expected = "vault: incomplete protocol exit")]
fn test_rebalance_fails_on_incomplete_protocol_exit() {
    // CRITICAL: If protocol exit fails or is partial, rebalance must abort
    // to prevent inconsistent state where funds are split between protocols
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    client.set_blend_pool(&owner, &blend_pool);

    // Deposit and rebalance to blend
    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);
    client.rebalance(&symbol_short!("blend"), &500_i128);

    // Verify funds are in blend
    assert_eq!(
        token_client.balance(&contract_id), 0,
        "Vault should have 0 after rebalance to blend"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        deposit_amount,
        "Blend should have full deposit amount"
    );

    // Set withdrawal limit to simulate stuck funds scenario
    // Pool has 10M, but can only withdraw 1M per transaction
    blend_client.set_max_withdraw_limit(&1_000_000_i128);

    // Attempt to rebalance to "none" - this should panic because:
    // - Expected to withdraw 10M from blend
    // - But pool can only return 1M per transaction
    // - 9M remains stuck in the protocol
    // - This is an incomplete exit and should fail
    client.rebalance(&symbol_short!("none"), &0_i128);

    // If we reach here, the test failed (rebalance should have panicked)
    panic!("Expected rebalance to panic on incomplete protocol exit, but it succeeded");
}

#[test]
#[should_panic(expected = "vault: incomplete protocol exit")]
fn test_rebalance_fails_when_switching_protocols_with_partial_exit() {
    // CRITICAL: When switching from blend to another protocol, complete exit is required
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    client.set_blend_pool(&owner, &blend_pool);

    // Deposit and rebalance to blend
    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);
    client.rebalance(&symbol_short!("blend"), &500_i128);

    // Verify funds are in blend
    assert_eq!(
        token_client.balance(&contract_id), 0,
        "Vault should have 0 after rebalance to blend"
    );
    assert_eq!(
        blend_client.supplied(&usdc_token),
        deposit_amount,
        "Blend should have full deposit amount"
    );

    // Set withdrawal limit to simulate stuck funds
    // Pool can only withdraw 2M per transaction, leaving 8M stuck
    blend_client.set_max_withdraw_limit(&2_000_000_i128);

    // Attempt to switch protocols (blend -> none) with limited withdrawal capacity
    // This should fail because we can't withdraw all funds from blend
    client.rebalance(&symbol_short!("none"), &0_i128);

    // If we reach here, the test failed
    panic!("Expected rebalance to panic on incomplete protocol exit when switching protocols");
}
