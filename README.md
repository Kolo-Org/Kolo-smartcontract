# Kolo Savings Platform - Smart Contract

This repository contains the core **Soroban Smart Contract** for the Kolo Savings Platform. Kolo is designed to facilitate Ajo/Esusu (rotational savings) directly on the Stellar blockchain, providing a trustless, transparent, and secure environment for community savings groups.
 
## Overview

The smart contract ensures strict adherence to rotational savings rules:
- **Fixed Payouts:** Enforces that payouts are exactly equal to the `contribution_amount * number_of_members`.
- **Fair Rotations:** Tracks which members have received payouts to guarantee each member is paid exactly once per cycle.
- **Trustless Execution:** Admin cannot arbitrarily withdraw funds or change payout amounts.

## Core Features

1. **Group Initialization**
   - Initializes a new savings group with a designated admin, a specific token (e.g., USDC), a group name, and a fixed contribution amount.
   
2. **Member Management**
   - The admin can add members to the group. Only registered members can contribute or receive payouts.

3. **Contributions**
   - Members contribute the exact fixed amount to the smart contract pool.

4. **Strict Payouts**
   - The admin triggers the payout to a specific member.
   - The contract verifies the recipient is a member, has not received a payout this cycle, and that the pool has sufficient funds.
   - The exact pooled amount is securely transferred to the recipient.

5. **Cycle Reset**
   - Once a cycle is complete, the admin can reset the cycle, allowing members to receive payouts in the next rotation.

## Prerequisites

To build and test the contract, you need to install Rust and the Soroban CLI:

1. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. Add the WebAssembly target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. Install the Soroban CLI:
   ```bash
   cargo install --locked soroban-cli
   ```

## Pre-commit Hooks

To ensure code quality, this repository uses pre-commit hooks to automatically format and lint Rust code before committing.

To install the hooks, run:

```bash
pip install pre-commit
pre-commit install
```

## Build

Compile the smart contract into a WebAssembly (`.wasm`) file:

```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
```

The compiled contract will be located at `contracts/target/wasm32-unknown-unknown/release/kolo_savings_group.wasm`.

## Test

Run the comprehensive Rust unit tests to verify the strict Ajo/Esusu logic:

```bash
cd contracts
cargo test
```

## Contract Methods

### Write Operations
- `initialize(admin: Address, token: Address, name: String, contribution_amount: i128)`
- `add_member(new_member: Address)` (Requires Admin Auth)
- `contribute(member: Address, amount: i128)` (Requires Member Auth)
- `payout(recipient: Address)` (Requires Admin Auth)
- `reset_cycle()` (Requires Admin Auth)

### Read Operations
- `get_balance() -> i128`
- `get_contribution(member: Address) -> i128`
- `has_received_payout(member: Address) -> bool`

## License

MIT License
