// boundless-profile: storage helpers.
//
// Storage layout (after the 2026-06 audit):
//
//   instance()    — admin + config (events binding, bootstrap default,
//                   paused, deployment seq). Single bag, auto-extended
//                   when we call touch_instance(env).
//   persistent()  — Profile(user) + EarningsByToken(user, token). Each
//                   read/write bumps TTL via touch_profile_persistent so
//                   active users stay live indefinitely.
//   temporary()   — OpSeen idempotency markers.
//
// Pre-audit this module placed everything in persistent. Same fix as the
// events contract.

#![allow(dead_code)]

use soroban_sdk::{Address, BytesN, Env};

use soroban_sdk::String;

use crate::errors::Error;
use crate::types::{DataKey, PendingAdmin, PendingEventsContract, PendingUpgrade, Profile};

// ============================================================
// TTL CONSTANTS (mainnet cadence ~5s/ledger)
// ============================================================
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_BUMP: u32 = 518_400;

const PROFILE_TTL_THRESHOLD: u32 = 86_400;
const PROFILE_TTL_BUMP: u32 = 1_555_200;

pub fn touch_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
}

fn touch_profile_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PROFILE_TTL_THRESHOLD, PROFILE_TTL_BUMP);
}

// ============================================================
// ADMIN / CONFIG (instance)
// ============================================================
pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_admin(env: &Env, addr: &Address) {
    env.storage().instance().set(&DataKey::Admin, addr);
}

pub fn get_pending_admin(env: &Env) -> Option<PendingAdmin> {
    env.storage().instance().get(&DataKey::PendingAdmin)
}

pub fn set_pending_admin(env: &Env, pending: &PendingAdmin) {
    env.storage()
        .instance()
        .set(&DataKey::PendingAdmin, pending);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingAdmin);
}

pub fn get_events_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::EventsContract)
}

pub fn set_events_contract(env: &Env, addr: &Address) {
    env.storage().instance().set(&DataKey::EventsContract, addr);
}

pub fn get_pending_events_contract(env: &Env) -> Option<PendingEventsContract> {
    env.storage()
        .instance()
        .get(&DataKey::PendingEventsContract)
}

pub fn set_pending_events_contract(env: &Env, pending: &PendingEventsContract) {
    env.storage()
        .instance()
        .set(&DataKey::PendingEventsContract, pending);
}

pub fn clear_pending_events_contract(env: &Env) {
    env.storage()
        .instance()
        .remove(&DataKey::PendingEventsContract);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

pub fn set_deployment_seq(env: &Env, seq: u32) {
    env.storage().instance().set(&DataKey::DeploymentSeq, &seq);
}

// ============================================================
// VERSION / UPGRADE / MIGRATION (instance; H6)
// ============================================================
pub fn get_version(env: &Env) -> Option<String> {
    env.storage().instance().get(&DataKey::Version)
}

pub fn set_version(env: &Env, version: &String) {
    env.storage().instance().set(&DataKey::Version, version);
}

pub fn get_pending_upgrade(env: &Env) -> Option<PendingUpgrade> {
    env.storage().instance().get(&DataKey::PendingUpgrade)
}

pub fn set_pending_upgrade(env: &Env, pending: &PendingUpgrade) {
    env.storage()
        .instance()
        .set(&DataKey::PendingUpgrade, pending);
}

pub fn clear_pending_upgrade(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingUpgrade);
}

pub fn get_migrated_to_version(env: &Env) -> Option<String> {
    env.storage().instance().get(&DataKey::MigratedToVersion)
}

pub fn set_migrated_to_version(env: &Env, version: &String) {
    env.storage()
        .instance()
        .set(&DataKey::MigratedToVersion, version);
}

// ============================================================
// PROFILE (persistent, per-user + per-read TTL bump)
// ============================================================
pub fn get_profile(env: &Env, user: &Address) -> Option<Profile> {
    let key = DataKey::Profile(user.clone());
    let profile: Option<Profile> = env.storage().persistent().get(&key);
    if profile.is_some() {
        touch_profile_persistent(env, &key);
    }
    profile
}

pub fn set_profile(env: &Env, user: &Address, profile: &Profile) {
    let key = DataKey::Profile(user.clone());
    env.storage().persistent().set(&key, profile);
    touch_profile_persistent(env, &key);
}

// ============================================================
// EARNINGS (persistent, per-user-per-token + per-read TTL bump)
// ============================================================
pub fn get_earnings(env: &Env, user: &Address, token: &Address) -> i128 {
    let key = DataKey::EarningsByToken(user.clone(), token.clone());
    let amt: Option<i128> = env.storage().persistent().get(&key);
    if amt.is_some() {
        touch_profile_persistent(env, &key);
    }
    amt.unwrap_or(0)
}

pub fn set_earnings(env: &Env, user: &Address, token: &Address, amount: i128) {
    let key = DataKey::EarningsByToken(user.clone(), token.clone());
    env.storage().persistent().set(&key, &amount);
    touch_profile_persistent(env, &key);
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
