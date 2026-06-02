// boundless-events: internal escrow / token movement helpers.
//
// All token-transfer operations route through here so that fee withholding
// and the atomic deposit pattern are in one place.
//
// Spec: boundless-platform-contract-prd.md Sections 6, 8.3.
//
// Wired by deposit / release / refund operations as they land.
#![allow(dead_code)]

use soroban_sdk::{token, Address, Env};

use crate::storage;

/// Compute the fee for a deposit amount given the current bps configuration.
pub fn compute_fee(env: &Env, amount: i128) -> i128 {
    let bps = storage::get_fee_bps(env) as i128;
    amount.saturating_mul(bps) / 10_000
}

/// Atomic deposit: pull `amount + fee` from `from`, then immediately forward
/// `fee` to the fee account. Returns `amount` (the net credited to the event).
pub fn deposit_with_fee(env: &Env, token_addr: &Address, from: &Address, amount: i128) -> i128 {
    let fee = compute_fee(env, amount);
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

/// Release `amount` of `token` to `recipient` from the contract's balance.
pub fn release(env: &Env, token_addr: &Address, recipient: &Address, amount: i128) {
    let contract = env.current_contract_address();
    let client = token::Client::new(env, token_addr);
    client.transfer(&contract, recipient, &amount);
}
