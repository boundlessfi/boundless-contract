// boundless-profile: storage helpers.
//
// dead_code allowed: setters wired by operation bodies.
#![allow(dead_code)]

use soroban_sdk::{Address, BytesN, Env};

use crate::errors::Error;
use crate::types::{DataKey, PendingAdmin, Profile};

// ============================================================
// ADMIN / CONFIG (persistent)
// ============================================================
pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(Error::AlreadyInitialized)
}

pub fn set_admin(env: &Env, addr: &Address) {
    env.storage().persistent().set(&DataKey::Admin, addr);
}

pub fn get_pending_admin(env: &Env) -> Option<PendingAdmin> {
    env.storage().persistent().get(&DataKey::PendingAdmin)
}

pub fn set_pending_admin(env: &Env, pending: &PendingAdmin) {
    env.storage()
        .persistent()
        .set(&DataKey::PendingAdmin, pending);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().persistent().remove(&DataKey::PendingAdmin);
}

pub fn get_events_contract(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::EventsContract)
}

pub fn set_events_contract(env: &Env, addr: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::EventsContract, addr);
}

pub fn get_default_bootstrap_credits(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::DefaultBootstrapCredits)
        .unwrap_or(0)
}

pub fn set_default_bootstrap_credits(env: &Env, amount: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::DefaultBootstrapCredits, &amount);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().persistent().set(&DataKey::Paused, &paused);
}

pub fn set_deployment_seq(env: &Env, seq: u32) {
    env.storage().persistent().set(&DataKey::DeploymentSeq, &seq);
}

// ============================================================
// PROFILE (persistent, per-user)
// ============================================================
pub fn get_profile(env: &Env, user: &Address) -> Option<Profile> {
    env.storage()
        .persistent()
        .get(&DataKey::Profile(user.clone()))
}

pub fn set_profile(env: &Env, user: &Address, profile: &Profile) {
    env.storage()
        .persistent()
        .set(&DataKey::Profile(user.clone()), profile);
}

// ============================================================
// EARNINGS (persistent, per-user-per-token)
// ============================================================
pub fn get_earnings(env: &Env, user: &Address, token: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::EarningsByToken(user.clone(), token.clone()))
        .unwrap_or(0)
}

pub fn set_earnings(env: &Env, user: &Address, token: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::EarningsByToken(user.clone(), token.clone()), &amount);
}

// ============================================================
// IDEMPOTENCY (temporary; auto-TTL)
// ============================================================
pub fn is_op_seen(env: &Env, op_id: &BytesN<32>) -> bool {
    env.storage()
        .temporary()
        .get(&DataKey::OpSeen(op_id.clone()))
        .unwrap_or(false)
}

pub fn mark_op_seen(env: &Env, op_id: &BytesN<32>) {
    env.storage()
        .temporary()
        .set(&DataKey::OpSeen(op_id.clone()), &true);
}
