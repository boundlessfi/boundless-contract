// boundless-events: idempotency helpers.
//
// Spec: boundless-platform-contract-prd.md Section 10.
//
// Wired by operation bodies as they land.
#![allow(dead_code)]

use soroban_sdk::{BytesN, Env};

use crate::errors::Error;
use crate::storage;

pub fn require_unseen(env: &Env, op_id: &BytesN<32>) -> Result<(), Error> {
    if storage::is_op_seen(env, op_id) {
        return Err(Error::OpAlreadySeen);
    }
    Ok(())
}

pub fn mark_seen(env: &Env, op_id: &BytesN<32>) {
    storage::mark_op_seen(env, op_id);
}

// Deployment-epoch ID base: upper 32 bits encode the ledger sequence at deploy
// time so that ids from different deployments never collide.
//
// Spec: boundless-platform-contract-prd.md Section 9.
pub fn id_base(env: &Env) -> u64 {
    let seq = storage::get_deployment_seq(env);
    (seq as u64) << 32
}

pub fn next_event_id(env: &Env) -> u64 {
    let base = id_base(env);
    let id = storage::get_next_event_id(env, base + 1);
    storage::set_next_event_id(env, id + 1);
    id
}

/// Tag constants for `derive_child`. One per cross-contract op kind so that
/// the events contract and the profile contract never share an OpSeen marker.
pub mod tag {
    pub const BOOTSTRAP: u8 = 0xB0;
    pub const SPEND_CREDITS: u8 = 0xC1;
    pub const REFUND_CREDITS: u8 = 0xC2;
    pub const EARN_CREDITS: u8 = 0xC3;
    pub const BUMP_REP: u8 = 0xD1;
    pub const SLASH_REP: u8 = 0xD2;
    pub const REGISTER_EARNINGS: u8 = 0xE1;
}

/// Derive a child op_id from a parent so that cross-contract calls within a
/// single events-side operation each have a unique idempotency marker.
///
/// XOR with a per-op tag in the first byte: cheap, deterministic, and the
/// orchestrator's sha256-based parent op_ids make collisions effectively
/// impossible.
pub fn derive_child(env: &Env, parent: &BytesN<32>, op_tag: u8) -> BytesN<32> {
    let mut payload = parent.to_array();
    payload[0] ^= op_tag;
    BytesN::from_array(env, &payload)
}

/// Same as `derive_child` but also XORs a sub-index into the second byte, so
/// per-winner cross-contract calls within select_winners get unique op_ids
/// even when the same op_tag is reused across winners.
pub fn derive_child_indexed(env: &Env, parent: &BytesN<32>, op_tag: u8, sub_idx: u8) -> BytesN<32> {
    let mut payload = parent.to_array();
    payload[0] ^= op_tag;
    payload[1] ^= sub_idx;
    BytesN::from_array(env, &payload)
}
