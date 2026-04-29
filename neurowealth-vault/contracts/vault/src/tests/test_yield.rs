//! Tests for yield / total-assets update functionality

use super::utils::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_agent_can_update_total_assets() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;

    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // Simulate yield: mint tokens to vault first so balance check passes
    let yield_amount = 5_000_000_i128;
    token_client.mint(&contract_id, &yield_amount);

    let new_total = deposit_amount + yield_amount;
    client.update_total_assets(&agent, &new_total, &false, &0);

    assert_eq!(client.get_total_assets(), new_total);
}

#[test]
#[should_panic(expected = "vault: only agent can update total assets")]
fn test_non_agent_cannot_update_total_assets() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let non_agent = Address::generate(&env);
    // update_total_assets asserts agent == stored_agent before anything else
    client.update_total_assets(&non_agent, &deposit_amount, &false, &0);
}

#[test]
fn test_yield_increases_user_asset_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let balance_before_yield = client.get_balance(&user);
    assert_eq!(balance_before_yield, deposit_amount);

    // Simulate 50% yield: mint tokens to vault first, then update the reported total
    let yield_amount = deposit_amount / 2;
    let new_total_assets = deposit_amount + yield_amount;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &new_total_assets, &false, &0);

    let balance_after_yield = client.get_balance(&user);
    assert!(
        balance_after_yield > balance_before_yield,
        "Balance should increase with yield"
    );
    assert_eq!(
        balance_after_yield, new_total_assets,
        "User should get full proportional share"
    );
}

#[test]
fn test_yield_distributed_proportionally_between_users() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let deposit1 = 10_000_000_i128;
    let deposit2 = 5_000_000_i128;

    mint_and_deposit(&env, &client, &usdc_token, &user1, deposit1);
    mint_and_deposit(&env, &client, &usdc_token, &user2, deposit2);

    let total_deposits = deposit1 + deposit2;

    // Simulate 50% yield: mint tokens first, then report new total
    let yield_amount = total_deposits / 2;
    let new_total_assets = total_deposits + yield_amount;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &new_total_assets, &false, &0);

    let balance1_after = client.get_balance(&user1);
    let balance2_after = client.get_balance(&user2);

    // User1 has 2/3 of total shares → gets 2/3 of new_total_assets
    // User2 has 1/3 of total shares → gets 1/3 of new_total_assets
    let expected_balance1 = deposit1 + (yield_amount * 2) / 3;
    let expected_balance2 = deposit2 + yield_amount / 3;

    // Allow ±1 stroop for integer rounding
    assert!(
        (balance1_after - expected_balance1).abs() <= 1,
        "User1 should get proportional yield"
    );
    assert!(
        (balance2_after - expected_balance2).abs() <= 1,
        "User2 should get proportional yield"
    );

    assert_eq!(
        balance1_after + balance2_after,
        new_total_assets,
        "Total balances should equal total assets"
    );
}

#[test]
fn test_yield_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    let yield_amount = 5_000_000_i128;
    let new_total = deposit_amount + yield_amount;
    token_client.mint(&contract_id, &yield_amount);
    client.update_total_assets(&agent, &new_total, &false, &0);

    let assets_events = find_events_by_topic(
        env.events().all(),
        &env,
        soroban_sdk::symbol_short!("assets"),
    );
    assert!(
        !assets_events.is_empty(),
        "Assets update should emit an event"
    );
}
