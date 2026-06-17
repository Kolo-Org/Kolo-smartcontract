#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Events, Address, Env, String, symbol_short, vec, FromVal, IntoVal};
use soroban_sdk::token;

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(&admin, &token, &name, &contribution_amount);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(&admin, &token, &name, &contribution_amount);
    client.initialize(&admin, &token, &name, &contribution_amount);
}

#[test]
fn test_add_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(&admin, &token, &name, &contribution_amount);

    env.mock_all_auths();
    let member1 = Address::generate(&env);
    client.add_member(&member1);
}

#[test]
#[should_panic(expected = "Not a member")]
fn test_contribute_not_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(&admin, &token, &name, &contribution_amount);

    env.mock_all_auths();
    let not_member = Address::generate(&env);
    client.contribute(&not_member, &1000);
}

#[test]
fn test_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    // 1. Test Initialize Event
    client.initialize(&admin, &token, &name, &contribution_amount);
    
    let events = env.events().all();
    assert_eq!(events.len(), 1);
    
    let init_event = events.get(0).unwrap();
    assert_eq!(init_event.0, contract_id);
    assert_eq!(
        init_event.1,
        vec![&env, symbol_short!("init").into_val(&env)]
    );
    let init_data: (Address, Address, String, i128) = <_>::from_val(&env, &init_event.2);
    assert_eq!(
        init_data,
        (admin.clone(), token.clone(), name.clone(), contribution_amount)
    );

    // 2. Test Add Member Event
    let member1 = Address::generate(&env);
    client.add_member(&member1);
    
    let events = env.events().all();
    assert_eq!(events.len(), 2); // 2 events now
    
    let add_mem_event = events.get(1).unwrap();
    assert_eq!(add_mem_event.0, contract_id);
    assert_eq!(
        add_mem_event.1,
        vec![&env, symbol_short!("add_mem").into_val(&env), member1.clone().into_val(&env)]
    );
    let add_mem_data: () = <_>::from_val(&env, &add_mem_event.2);
    assert_eq!(add_mem_data, ());
}
