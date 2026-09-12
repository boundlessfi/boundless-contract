#![allow(dead_code)]

use soroban_sdk::{xdr::ToXdr, Address, Bytes, BytesN, Env};

use crate::errors::Error;
use crate::storage;

// OpSeen is namespaced by the authorizing caller so that a permissionless
// entrypoint (submit, apply, add_funds) cannot pre-mark an op_id and block a
// privileged one (select_winners, claim, cancel) that shares it. Pass the
// address that require_auth'd this call as the domain.
pub fn require_unseen(env: &Env, domain: &Address, op_id: &BytesN<32>) -> Result<(), Error> {
    if storage::is_op_seen(env, domain, op_id) {
        return Err(Error::OpAlreadySeen);
    }
    Ok(())
}

pub fn mark_seen(env: &Env, domain: &Address, op_id: &BytesN<32>) {
    storage::mark_op_seen(env, domain, op_id);
}

pub fn id_base(env: &Env) -> u64 {
    let seq = storage::get_deployment_seq(env);
    (seq as u64) << 32
}

pub fn next_event_id(env: &Env) -> Result<u64, Error> {
    let base = id_base(env);
    let fallback = base.checked_add(1).ok_or(Error::EventIdOverflow)?;
    let id = storage::get_next_event_id(env, fallback);
    let next = id.checked_add(1).ok_or(Error::EventIdOverflow)?;
    storage::set_next_event_id(env, next);
    Ok(id)
}

pub mod tag {
    pub const BOOTSTRAP: u8 = 0xB0;
    pub const BUMP_REP: u8 = 0xD1;
    pub const SLASH_REP: u8 = 0xD2;
    pub const REGISTER_EARNINGS: u8 = 0xE1;
}

/// Collision-resistant child op_id.
///
/// Invariant: `sha256(parent ‖ op_tag ‖ sub_idx ‖ domain ‖ callee_contract_id)`.
/// XOR into parent bytes is malleable (reversible, squat-friendly); hashing with
/// the authorizing domain address and profile contract id as domain separators is not.
pub fn derive_child(env: &Env, domain: &Address, parent: &BytesN<32>, op_tag: u8) -> BytesN<32> {
    derive_child_indexed(env, domain, parent, op_tag, 0)
}

pub fn derive_child_indexed(
    env: &Env,
    domain: &Address,
    parent: &BytesN<32>,
    op_tag: u8,
    sub_idx: u8,
) -> BytesN<32> {
    let callee = storage::get_profile_contract(env);
    let mut payload = Bytes::new(env);
    payload.append(&Bytes::from_array(env, &parent.to_array()));
    payload.push_back(op_tag);
    payload.push_back(sub_idx);
    payload.append(&domain.to_xdr(env));
    payload.append(&callee.to_xdr(env));
    let digest = env.crypto().sha256(&payload);
    BytesN::from_array(env, &digest.to_array())
}
