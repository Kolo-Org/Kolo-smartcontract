#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, String, Vec,
};

mod test;

const LEDGERS_TO_LIVE: u32 = 518_400; // ~30 days at 5s/ledger

fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Name,
    ContributionAmount,
    Members,
    Contributions(Address),
    HasReceivedPayout(Address),
    HasContributedThisCycle(Address),
    CycleMemberCount,
    User(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct User {
    pub wallet_address: Address,
    pub joined_groups: Vec<u32>,
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

        admin.require_auth();
        extend_instance_ttl(&env);

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::ContributionAmount, &contribution_amount);
        
        let empty_members: Vec<Address> = Vec::new(&env);
        env.storage().instance().set(&DataKey::Members, &empty_members);

        env.events().publish((symbol_short!("init"),), (admin, token, name, contribution_amount));
    }

    /// Add a member to the group (Admin only)
    pub fn add_member(env: Env, new_member: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&new_member) {
            members.push_back(new_member.clone());
            env.storage().instance().set(&DataKey::Members, &members);
            env.storage().persistent().set(&DataKey::Contributions(new_member.clone()), &0i128);
            env.storage().persistent().set(&DataKey::HasReceivedPayout(new_member.clone()), &false);
            env.storage().persistent().set(&DataKey::HasContributedThisCycle(new_member.clone()), &false);

            env.events().publish((symbol_short!("add_mem"), new_member), ());
        }
    }

    /// Contribute to the pool
    pub fn contribute(env: Env, member: Address, amount: i128) {
        member.require_auth();
        extend_instance_ttl(&env);

        let expected_amount: i128 = env.storage().instance().get(&DataKey::ContributionAmount).unwrap();
        if amount != expected_amount {
            panic!("Must contribute the exact amount");
        }

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member) {
            panic!("Not a member");
        }

        // Freeze the member count at the start of a cycle on the first contribution
        if !env.storage().instance().has(&DataKey::CycleMemberCount) {
            let count = members.len() as i128;
            env.storage().instance().set(&DataKey::CycleMemberCount, &count);
        }

        let has_contributed: bool = env.storage().persistent()
            .get(&DataKey::HasContributedThisCycle(member.clone()))
            .unwrap_or(false);
        if has_contributed {
            panic!("Already contributed this cycle");
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);

        // Transfer tokens from the member to this contract
        token_client.transfer(&member, &env.current_contract_address(), &amount);

        env.storage().persistent().set(&DataKey::HasContributedThisCycle(member.clone()), &true);

        let current_contribution: i128 = env.storage().persistent().get(&DataKey::Contributions(member.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Contributions(member.clone()), &(current_contribution + amount));

        env.storage().persistent().extend_ttl(&DataKey::Contributions(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
        env.storage().persistent().extend_ttl(&DataKey::HasContributedThisCycle(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);

        env.events().publish((symbol_short!("contrib"), member), amount);
    }

    /// Withdraw payout (Admin triggers payout to a member)
    /// Enforces strictly fixed rotational payout (Ajo/Esusu) rules.
    pub fn payout(env: Env, recipient: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&recipient) {
            panic!("Recipient is not a member");
        }

        let has_received: bool = env.storage().persistent().get(&DataKey::HasReceivedPayout(recipient.clone())).unwrap_or(false);
        if has_received {
            panic!("Recipient has already received a payout this cycle");
        }

        let contribution_amount: i128 = env.storage().instance().get(&DataKey::ContributionAmount).unwrap();
        let frozen_count: i128 = env.storage().instance()
            .get(&DataKey::CycleMemberCount)
            .expect("No active cycle");
        let pool_size = contribution_amount * frozen_count;

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        
        let contract_balance = token_client.balance(&env.current_contract_address());
        if pool_size > contract_balance {
            panic!("Insufficient funds in contract for full payout");
        }

        env.storage().persistent().set(&DataKey::HasReceivedPayout(recipient.clone()), &true);
        env.storage().persistent().extend_ttl(&DataKey::HasReceivedPayout(recipient.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
        token_client.transfer(&env.current_contract_address(), &recipient, &pool_size);

        env.events().publish((symbol_short!("payout"), recipient), pool_size);
    }

    /// Resets the payout cycle so members can receive payouts again.
    pub fn reset_cycle(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        for member in members.iter() {
            env.storage().persistent().set(&DataKey::HasReceivedPayout(member.clone()), &false);
            env.storage().persistent().set(&DataKey::HasContributedThisCycle(member.clone()), &false);
            env.storage().persistent().extend_ttl(&DataKey::HasReceivedPayout(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
            env.storage().persistent().extend_ttl(&DataKey::HasContributedThisCycle(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
        }

        // Clear the frozen member count so it is re-established at the next cycle's first contribution
        env.storage().instance().remove(&DataKey::CycleMemberCount);

        env.events().publish((symbol_short!("reset"),), ());
    }

    /// Get contract balance
    pub fn get_balance(env: Env) -> i128 {
        extend_instance_ttl(&env);
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.balance(&env.current_contract_address())
    }

    pub fn get_contribution(env: Env, member: Address) -> i128 {
        env.storage().persistent().extend_ttl(&DataKey::Contributions(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
        env.storage().persistent().get(&DataKey::Contributions(member)).unwrap_or(0)
    }

    pub fn has_received_payout(env: Env, member: Address) -> bool {
        env.storage().persistent().extend_ttl(&DataKey::HasReceivedPayout(member.clone()), LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
        env.storage().persistent().get(&DataKey::HasReceivedPayout(member)).unwrap_or(false)
    }
}
