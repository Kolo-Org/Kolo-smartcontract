//! Tests for vault initialization

use super::utils::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_initialize_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, NeuroWealthVault);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let deployer = Address::generate(&env);
    let agent = Address::generate(&env);
    let owner = Address::generate(&env);
    let usdc_token = Address::generate(&env);

    // Deployer must call initialize to prevent front-running
    client.initialize(&deployer, &owner, &agent, &usdc_token);

    // Verify initialization
    assert_eq!(client.get_agent(), agent);
    assert_eq!(client.get_usdc_token(), usdc_token);
    assert_eq!(client.get_owner(), owner);
    assert!(!client.is_paused());
    assert_eq!(client.get_version(), 1u32);
    assert_eq!(client.get_total_deposits(), 0);
    assert_eq!(client.get_total_assets(), 0);
}

#[test]
#[should_panic(expected = "vault: already initialized")]
fn test_double_initialize_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, NeuroWealthVault);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let deployer = Address::generate(&env);
    let agent = Address::generate(&env);
    let owner = Address::generate(&env);
    let usdc_token = Address::generate(&env);

    client.initialize(&deployer, &owner, &agent, &usdc_token);
    // Second call should panic with "vault: already initialized"
    client.initialize(&deployer, &owner, &agent, &usdc_token);
}

#[test]
fn test_initialize_default_values() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Verify actual defaults set by initialize()
    assert!(!client.is_paused(), "Vault should start unpaused");
    assert_eq!(
        client.get_min_deposit(),
        1_000_000_i128,
        "Default min deposit should be 1 USDC"
    );
    assert_eq!(
        client.get_max_deposit(),
        10_000_000_000_i128,
        "Default max deposit should be 10K USDC"
    );
    // TvLCap and UserDepositCap are set to non-zero defaults by initialize()
    assert_eq!(
        client.get_tvl_cap(),
        100_000_000_000_i128,
        "Default TVL cap is 100M USDC"
    );
    assert_eq!(
        client.get_user_deposit_cap(),
        10_000_000_000_i128,
        "Default user deposit cap is 10K USDC"
    );
    assert_eq!(
        client.get_total_deposits(),
        0,
        "Initial deposits should be 0"
    );
    assert_eq!(client.get_total_assets(), 0, "Initial assets should be 0");
}

#[test]
fn test_initialize_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, NeuroWealthVault);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let agent = Address::generate(&env);
    let owner = Address::generate(&env);
    let usdc_token = Address::generate(&env);

    let deployer = Address::generate(&env);
    client.initialize(&deployer, &owner, &agent, &usdc_token);

    let events = env.events().all();
    assert!(!events.is_empty(), "Initialization should emit an event");

    let init_events =
        find_events_by_topic(env.events().all(), &env, soroban_sdk::symbol_short!("init"));
    assert!(!init_events.is_empty(), "Should have initialization event");
}
