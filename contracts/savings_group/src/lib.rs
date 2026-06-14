#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String, Vec,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Name,
    ContributionAmount,
    Members,
    Contributions(Address),
}

#[contract]
pub struct KoloSavingsContract;

#[contractimpl]
impl KoloSavingsContract {
    /// Initialize the savings group
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        name: String,
        contribution_amount: i128,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::ContributionAmount, &contribution_amount);
        
        let empty_members: Vec<Address> = Vec::new(&env);
        env.storage().instance().set(&DataKey::Members, &empty_members);
    }

    /// Add a member to the group (Admin only)
    pub fn add_member(env: Env, new_member: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&new_member) {
            members.push_back(new_member.clone());
            env.storage().instance().set(&DataKey::Members, &members);
            env.storage().persistent().set(&DataKey::Contributions(new_member), &0i128);
        }
    }

    /// Contribute to the pool
    pub fn contribute(env: Env, member: Address, amount: i128) {
        member.require_auth();

        let expected_amount: i128 = env.storage().instance().get(&DataKey::ContributionAmount).unwrap();
        if amount != expected_amount {
            panic!("Must contribute the exact amount");
        }

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member) {
            panic!("Not a member");
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        
        // Transfer tokens from the member to this contract
        token_client.transfer(&member, &env.current_contract_address(), &amount);

        // Update member's contribution total
        let current_contribution: i128 = env.storage().persistent().get(&DataKey::Contributions(member.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Contributions(member), &(current_contribution + amount));
    }

    /// Withdraw payout (Admin triggers payout to a member)
    pub fn payout(env: Env, recipient: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&recipient) {
            panic!("Recipient is not a member");
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        
        let contract_balance = token_client.balance(&env.current_contract_address());
        if amount > contract_balance {
            panic!("Insufficient funds in contract");
        }

        // Transfer tokens from contract to recipient
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);
    }

    /// Get contract balance
    pub fn get_balance(env: Env) -> i128 {
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.balance(&env.current_contract_address())
    }

    /// Get a member's total contributions
    pub fn get_contribution(env: Env, member: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Contributions(member)).unwrap_or(0)
    }
}

mod test;
