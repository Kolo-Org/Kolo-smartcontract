//! Tests verifying that each contract operation emits the expected event with correct payload values

use super::utils::*;
use crate::{
    AgentUpdatedEvent, AssetsUpdatedEvent, DepositEvent, EmergencyPausedEvent, LimitsUpdatedEvent,
    RebalanceEvent, VaultInitializedEvent, VaultPausedEvent, VaultUnpausedEvent, WithdrawEvent,
};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, TryFromVal};

#[test]
fn test_initialize_emits_init_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, NeuroWealthVault);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let agent = Address::generate(&env);
    let owner = Address::generate(&env);
    let usdc_token = Address::generate(&env);
    let expected_tvl_cap = 100_000_000_000_i128;
    let deployer = Address::generate(&env);
    client.initialize(&deployer, &owner, &agent, &usdc_token);

    let init_events = find_events_by_topic(env.events().all(), &env, symbol_short!("init"));
    assert_eq!(
        init_events.len(),
        1,
        "Exactly one init event should be emitted"
    );

    let (_, _, data) = &init_events[0];
    let event =
        VaultInitializedEvent::try_from_val(&env, data).expect("Should be a VaultInitializedEvent");

    assert_eq!(
        event.agent, agent,
        "Event agent should match initialized agent"
    );
    assert_eq!(
        event.usdc_token, usdc_token,
        "Event usdc_token should match initialized token"
    );
    assert_eq!(
        event.tvl_cap, expected_tvl_cap,
        "Event tvl_cap should match default cap"
    );
}

#[test]
fn test_deposit_emits_deposit_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 5_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let deposit_events = find_events_by_topic(env.events().all(), &env, symbol_short!("deposit"));
    assert!(!deposit_events.is_empty(), "Deposit should emit an event");

    let (_, _, data) = &deposit_events[0];
    let event = DepositEvent::try_from_val(&env, data).expect("Should be a DepositEvent");

    assert_eq!(event.user, user, "Event user should match depositor");
    assert_eq!(
        event.amount, deposit_amount,
        "Event amount should match deposited amount"
    );
    assert_eq!(
        event.shares, deposit_amount,
        "First deposit should mint 1:1 shares"
    );
}

#[test]
fn test_deposit_multiple_users_events_correct() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let amount1 = 10_000_000_i128;
    let amount2 = 5_000_000_i128;

    mint_and_deposit(&env, &client, &usdc_token, &user1, amount1);
    mint_and_deposit(&env, &client, &usdc_token, &user2, amount2);

    let deposit_events = find_events_by_topic(env.events().all(), &env, symbol_short!("deposit"));
    assert_eq!(deposit_events.len(), 2, "Should emit two deposit events");

    let (_, _, data1) = &deposit_events[0];
    let event1 = DepositEvent::try_from_val(&env, data1).expect("Should be a DepositEvent");
    assert_eq!(event1.user, user1);
    assert_eq!(event1.amount, amount1);
    assert_eq!(event1.shares, amount1);

    let (_, _, data2) = &deposit_events[1];
    let event2 = DepositEvent::try_from_val(&env, data2).expect("Should be a DepositEvent");
    assert_eq!(event2.user, user2);
    assert_eq!(event2.amount, amount2);
    assert_eq!(event2.shares, amount2);
}

#[test]
fn test_withdraw_emits_withdraw_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let withdraw_amount = 3_000_000_i128;
    client.withdraw(&user, &withdraw_amount);

    let withdraw_events = find_events_by_topic(env.events().all(), &env, symbol_short!("withdraw"));
    assert!(!withdraw_events.is_empty(), "Withdraw should emit an event");

    let (_, _, data) = &withdraw_events[0];
    let event = WithdrawEvent::try_from_val(&env, data).expect("Should be a WithdrawEvent");

    assert_eq!(event.user, user, "Event user should match withdrawer");
    assert_eq!(
        event.amount, withdraw_amount,
        "Event amount should match withdrawn amount"
    );
    assert_eq!(
        event.shares, withdraw_amount,
        "At 1:1 price, shares burned should equal amount"
    );
}

#[test]
fn test_withdraw_all_emits_withdraw_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let withdrawn = client.withdraw_all(&user);

    assert_eq!(withdrawn, deposit_amount, "Should withdraw full balance");

    let withdraw_events = find_events_by_topic(env.events().all(), &env, symbol_short!("withdraw"));
    let last_event_data = &withdraw_events.last().unwrap().2;
    let event =
        WithdrawEvent::try_from_val(&env, last_event_data).expect("Should be a WithdrawEvent");

    assert_eq!(event.user, user, "Event user should match withdrawer");
    assert_eq!(
        event.amount, deposit_amount,
        "Event amount should match full balance"
    );
    assert_eq!(event.shares, deposit_amount, "Should burn all shares");
}

#[test]
fn test_pause_emits_paused_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);

    let pause_events = find_events_by_topic(env.events().all(), &env, symbol_short!("paused"));
    assert!(!pause_events.is_empty(), "Pause should emit an event");

    let (_, _, data) = &pause_events[0];
    let event = VaultPausedEvent::try_from_val(&env, data).expect("Should be a VaultPausedEvent");
    assert_eq!(event.owner, owner, "Event owner should match pauser");
}

#[test]
fn test_unpause_emits_unpaused_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    client.unpause(&owner);

    let unpause_events = find_events_by_topic(env.events().all(), &env, symbol_short!("unpaused"));
    assert!(!unpause_events.is_empty(), "Unpause should emit an event");

    let (_, _, data) = &unpause_events[0];
    let event =
        VaultUnpausedEvent::try_from_val(&env, data).expect("Should be a VaultUnpausedEvent");
    assert_eq!(event.owner, owner, "Event owner should match unpauser");
}

#[test]
fn test_emergency_pause_emits_emergency_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.emergency_pause(&agent);

    let emergency_events = find_events_by_topic(env.events().all(), &env, symbol_short!("emerg"));
    assert!(
        !emergency_events.is_empty(),
        "Emergency pause should emit an event"
    );

    let (_, _, data) = &emergency_events[0];
    let event =
        EmergencyPausedEvent::try_from_val(&env, data).expect("Should be an EmergencyPausedEvent");
    assert_eq!(
        event.owner, agent,
        "Event owner should match emergency pauser"
    );
}

#[test]
fn test_set_deposit_limits_emits_limits_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_min = 2_000_000_i128;
    let new_max = 20_000_000_000_i128;
    client.set_deposit_limits(&new_min, &new_max);

    let limits_events = find_events_by_topic(env.events().all(), &env, symbol_short!("l_upd"));
    assert!(
        !limits_events.is_empty(),
        "set_deposit_limits should emit a limits event"
    );

    let (_, _, data) = &limits_events[0];
    let event =
        LimitsUpdatedEvent::try_from_val(&env, data).expect("Should be a LimitsUpdatedEvent");
    assert_eq!(
        event.new_min, new_min,
        "Event new_min should match set value"
    );
    assert_eq!(
        event.new_max, new_max,
        "Event new_max should match set value"
    );
}

#[test]
fn test_update_agent_emits_agent_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, old_agent, _owner, _usdc_token) = setup_vault(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent = Address::generate(&env);
    client.update_agent(&new_agent);

    let agent_events = find_events_by_topic(env.events().all(), &env, symbol_short!("agent"));
    assert!(
        !agent_events.is_empty(),
        "update_agent should emit an event"
    );

    let (_, _, data) = &agent_events[0];
    let event =
        AgentUpdatedEvent::try_from_val(&env, data).expect("Should be an AgentUpdatedEvent");
    assert_eq!(
        event.old_agent, old_agent,
        "Event old_agent should match previous agent"
    );
    assert_eq!(
        event.new_agent, new_agent,
        "Event new_agent should match new agent"
    );
}

#[test]
fn test_update_total_assets_emits_assets_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let old_total = deposit_amount;
    let yield_amount = 5_000_000_i128;
    let new_total = old_total + yield_amount;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &new_total, &false, &0);

    let assets_events = find_events_by_topic(env.events().all(), &env, symbol_short!("assets"));
    assert!(
        !assets_events.is_empty(),
        "update_total_assets should emit an event"
    );

    let (_, _, data) = &assets_events[0];
    let event =
        AssetsUpdatedEvent::try_from_val(&env, data).expect("Should be an AssetsUpdatedEvent");
    assert_eq!(
        event.old_total, old_total,
        "Event old_total should match previous total"
    );
    assert_eq!(
        event.new_total, new_total,
        "Event new_total should match new total"
    );
}

#[test]
fn test_rebalance_emits_rebalance_event_with_correct_payload() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let expected_apy = 850_i128;
    client.rebalance(&symbol_short!("none"), &expected_apy);

    let rebalance_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("rebalance"));
    assert!(
        !rebalance_events.is_empty(),
        "rebalance should emit an event"
    );

    let (_, _, data) = &rebalance_events[0];
    let event = RebalanceEvent::try_from_val(&env, data).expect("Should be a RebalanceEvent");
    assert_eq!(
        event.protocol,
        symbol_short!("none"),
        "Event protocol should match rebalance target"
    );
    assert_eq!(
        event.expected_apy, expected_apy,
        "Event expected_apy should match provided APY"
    );
}

#[test]
fn test_rebalance_with_blend_emits_correct_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.set_blend_pool(&agent, &blend_pool);

    let user = Address::generate(&env);
    mint_and_deposit(&env, &client, &usdc_token, &user, 10_000_000_i128);

    let expected_apy = 1200_i128;
    client.rebalance(&symbol_short!("blend"), &expected_apy);

    let rebalance_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("rebalance"));
    let last_event_data = &rebalance_events.last().unwrap().2;
    let event =
        RebalanceEvent::try_from_val(&env, last_event_data).expect("Should be a RebalanceEvent");
    assert_eq!(
        event.protocol,
        symbol_short!("blend"),
        "Event protocol should be blend"
    );
    assert_eq!(
        event.expected_apy, expected_apy,
        "Event expected_apy should match provided APY"
    );
}

#[test]
fn test_all_events_have_correct_topics() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.set_deposit_limits(&1_000_000_i128, &10_000_000_000_i128);
    client.rebalance(&symbol_short!("none"), &500_i128);
    client.pause(&owner);
    client.unpause(&owner);
    client.emergency_pause(&agent);

    let init_events = find_events_by_topic(env.events().all(), &env, symbol_short!("init"));
    let limits_events = find_events_by_topic(env.events().all(), &env, symbol_short!("l_upd"));
    let rebalance_events =
        find_events_by_topic(env.events().all(), &env, symbol_short!("rebalance"));
    let paused_events = find_events_by_topic(env.events().all(), &env, symbol_short!("paused"));
    let unpaused_events = find_events_by_topic(env.events().all(), &env, symbol_short!("unpaused"));
    let emerg_events = find_events_by_topic(env.events().all(), &env, symbol_short!("emerg"));

    assert!(!init_events.is_empty(), "Should have init events");
    assert!(!limits_events.is_empty(), "Should have limits events");
    assert!(!rebalance_events.is_empty(), "Should have rebalance events");
    assert!(!paused_events.is_empty(), "Should have paused events");
    assert!(!unpaused_events.is_empty(), "Should have unpaused events");
    assert!(
        !emerg_events.is_empty(),
        "Should have emergency paused events"
    );

    for (addr, topics, _) in env.events().all().iter() {
        assert_eq!(
            addr, contract_id,
            "All events should be from vault contract"
        );
        assert!(
            !topics.is_empty(),
            "Each event should have at least one topic"
        );
    }
}
