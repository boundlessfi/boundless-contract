// boundless-profile: reputation operations.
//
// Spec: boundless-credits-reputation-prd.md Section 5.3.

use soroban_sdk::{Address, BytesN, Env, String, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency;
use crate::storage;

pub fn bump(
    env: &Env,
    user: Address,
    delta: u32,
    reason: Symbol,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    profile.reputation = profile.reputation.saturating_add(delta as u64);
    storage::set_profile(env, &user, &profile);

    evt::ReputationBumped {
        user,
        delta,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn slash(
    env: &Env,
    user: Address,
    delta: u32,
    reason: Symbol,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    profile.reputation = profile.reputation.saturating_sub(delta as u64);
    storage::set_profile(env, &user, &profile);

    evt::ReputationSlashed {
        user,
        delta,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn admin_slash(
    env: &Env,
    user: Address,
    delta: u32,
    reason: String,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_admin(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if reason.len() == 0 {
        return Err(Error::ReasonRequired);
    }

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    profile.reputation = profile.reputation.saturating_sub(delta as u64);
    storage::set_profile(env, &user, &profile);

    evt::AdminReputationSlashed {
        user,
        delta,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}
