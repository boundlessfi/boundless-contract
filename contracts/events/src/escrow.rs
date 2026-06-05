#![allow(dead_code)]

use soroban_sdk::{token, Address, Env};

use crate::storage;

pub fn effective_fee_bps(env: &Env, override_bps: Option<u32>) -> u32 {
    override_bps.unwrap_or_else(|| storage::get_fee_bps(env))
}

pub fn compute_fee_at(amount: i128, bps: u32) -> i128 {
    amount.saturating_mul(bps as i128) / 10_000
}

pub fn compute_fee(env: &Env, amount: i128) -> i128 {
    compute_fee_at(amount, storage::get_fee_bps(env))
}

pub fn deposit_with_fee_at(
    env: &Env,
    token_addr: &Address,
    from: &Address,
    amount: i128,
    bps: u32,
) -> i128 {
    let fee = compute_fee_at(amount, bps);
    let total = amount.saturating_add(fee);
    let contract = env.current_contract_address();
    let fee_account = storage::get_fee_account(env);

    let client = token::Client::new(env, token_addr);
    client.transfer(from, &contract, &total);
    if fee > 0 {
        client.transfer(&contract, &fee_account, &fee);
    }
    amount
}

pub fn deposit_with_fee(env: &Env, token_addr: &Address, from: &Address, amount: i128) -> i128 {
    deposit_with_fee_at(env, token_addr, from, amount, storage::get_fee_bps(env))
}

pub fn release(env: &Env, token_addr: &Address, recipient: &Address, amount: i128) {
    let contract = env.current_contract_address();
    let client = token::Client::new(env, token_addr);
    client.transfer(&contract, recipient, &amount);
}
