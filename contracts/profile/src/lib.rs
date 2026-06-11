// SPDX-License-Identifier: MIT
//
// boundless-profile
//
// Per-user credits + reputation + per-token earnings. Mutated almost
// exclusively by the events contract; admin can grant credits and slash
// reputation directly with audited reasons.
//
// Spec: boundless-credits-reputation-prd.md
#![no_std]

use soroban_sdk::{contract, contractimpl, contractmeta, Address, BytesN, Env, String, Symbol};

mod admin;
mod credits;
mod earnings;
mod errors;
mod events;
mod idempotency;
mod reputation;
mod storage;
mod types;

#[cfg(test)]
mod tests;

use crate::errors::Error;
use crate::types::{PendingEventsContract, PendingUpgrade, Profile};

contractmeta!(key = "version", val = "0.1.0");
contractmeta!(
    key = "description",
    val = "Boundless profile contract: credits + reputation"
);
contractmeta!(key = "license", val = "MIT");

#[contract]
pub struct ProfileContract;

#[contractimpl]
impl ProfileContract {
    // ============================================================
    // CONSTRUCTOR
    // ============================================================
    pub fn __constructor(env: Env, admin: Address, default_bootstrap_credits: u32) {
        admin::initialize(&env, admin, default_bootstrap_credits);
    }

    // ============================================================
    // ADMIN
    // ============================================================
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        admin::set_admin(&env, new_admin)
    }

    pub fn accept_admin(env: Env) -> Result<(), Error> {
        admin::accept_admin(&env)
    }

    pub fn set_events_contract(env: Env, new_addr: Address) -> Result<(), Error> {
        admin::set_events_contract(&env, new_addr)
    }

    pub fn propose_events_contract(env: Env, new_addr: Address) -> Result<(), Error> {
        admin::propose_events_contract(&env, new_addr)
    }

    pub fn accept_events_contract(env: Env) -> Result<(), Error> {
        admin::accept_events_contract(&env)
    }

    pub fn cancel_pending_events_contract(env: Env) -> Result<(), Error> {
        admin::cancel_pending_events_contract(&env)
    }

    pub fn set_default_bootstrap_credits(env: Env, new_amount: u32) -> Result<(), Error> {
        admin::set_default_bootstrap_credits(&env, new_amount)
    }

    pub fn pause(env: Env) -> Result<(), Error> {
        admin::pause(&env)
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        admin::unpause(&env)
    }

    pub fn propose_upgrade(
        env: Env,
        new_wasm_hash: BytesN<32>,
        new_version: String,
    ) -> Result<(), Error> {
        admin::propose_upgrade(&env, new_wasm_hash, new_version)
    }

    pub fn apply_upgrade(env: Env) -> Result<(), Error> {
        admin::apply_upgrade(&env)
    }

    pub fn cancel_pending_upgrade(env: Env) -> Result<(), Error> {
        admin::cancel_pending_upgrade(&env)
    }

    pub fn migrate(env: Env) -> Result<(), Error> {
        admin::migrate(&env)
    }

    // ============================================================
    // BOOTSTRAP
    // ============================================================
    pub fn bootstrap(env: Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
        credits::bootstrap(&env, user, op_id)
    }

    /// Self-service profile creation: the user authorizes their own bootstrap
    /// (no admin key, no events-contract dependency). Called at onboarding so
    /// every user has a profile before they participate.
    pub fn bootstrap_self(
        env: Env,
        user: Address,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        credits::bootstrap_self(&env, user, op_id)
    }

    // ============================================================
    // CREDITS
    // ============================================================
    pub fn spend_credits(
        env: Env,
        user: Address,
        amount: u32,
        reason: Symbol,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        credits::spend(&env, user, amount, reason, op_id)
    }

    pub fn earn_credits(
        env: Env,
        user: Address,
        amount: u32,
        reason: Symbol,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        credits::earn(&env, user, amount, reason, op_id)
    }

    pub fn refund_credits(
        env: Env,
        user: Address,
        amount: u32,
        reason: Symbol,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        credits::refund(&env, user, amount, reason, op_id)
    }

    // ============================================================
    // REPUTATION
    // ============================================================
    pub fn bump_reputation(
        env: Env,
        user: Address,
        delta: u32,
        reason: Symbol,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        reputation::bump(&env, user, delta, reason, op_id)
    }

    pub fn slash_reputation(
        env: Env,
        user: Address,
        delta: u32,
        reason: Symbol,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        reputation::slash(&env, user, delta, reason, op_id)
    }

    // ============================================================
    // EARNINGS
    // ============================================================
    pub fn register_earnings(
        env: Env,
        user: Address,
        token: Address,
        amount: i128,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        earnings::register(&env, user, token, amount, op_id)
    }

    // ============================================================
    // ADMIN-DIRECT MUTATIONS
    // ============================================================
    pub fn admin_grant_credits(
        env: Env,
        user: Address,
        amount: u32,
        reason: String,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        credits::admin_grant(&env, user, amount, reason, op_id)
    }

    pub fn admin_slash_reputation(
        env: Env,
        user: Address,
        delta: u32,
        reason: String,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        reputation::admin_slash(&env, user, delta, reason, op_id)
    }

    // ============================================================
    // READS
    // ============================================================
    pub fn get_profile(env: Env, user: Address) -> Option<Profile> {
        storage::get_profile(&env, &user)
    }

    pub fn get_earnings(env: Env, user: Address, token: Address) -> i128 {
        storage::get_earnings(&env, &user, &token)
    }

    pub fn get_admin(env: Env) -> Address {
        admin::get_admin(&env)
    }

    pub fn get_events_contract(env: Env) -> Option<Address> {
        admin::get_events_contract(&env)
    }

    pub fn get_pending_events_contract(env: Env) -> Option<PendingEventsContract> {
        admin::get_pending_events_contract(&env)
    }

    pub fn get_default_bootstrap_credits(env: Env) -> u32 {
        admin::get_default_bootstrap_credits(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        admin::is_paused(&env)
    }

    pub fn version(env: Env) -> String {
        admin::get_version(&env)
    }

    pub fn get_pending_upgrade(env: Env) -> Option<PendingUpgrade> {
        admin::get_pending_upgrade(&env)
    }

    pub fn get_migrated_to_version(env: Env) -> Option<String> {
        admin::get_migrated_to_version(&env)
    }
}
