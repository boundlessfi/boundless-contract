// boundless-profile: credit operations.
//
// Spec: boundless-credits-reputation-prd.md Section 5.2, 5.5.

use soroban_sdk::{Address, BytesN, Env, String, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency;
use crate::storage;
use crate::types::Profile;

/// Lazy bootstrap. Called by the events contract on a user's first touch.
/// Idempotent: a second call when the profile already exists is a no-op.
pub fn bootstrap(env: &Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if storage::get_profile(env, &user).is_none() {
        let initial = storage::get_default_bootstrap_credits(env);
        let profile = Profile::new(env.ledger().timestamp(), initial);
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped {
            user,
            initial_credits: initial,
        }
        .publish(env);
    }

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

/// Self-service bootstrap. A user creates their OWN profile by authorizing the
/// call with their wallet — no admin key, no events-contract dependency. Used
/// at platform onboarding so every user has a profile before they participate.
/// Idempotent: a second call when the profile already exists is a no-op.
pub fn bootstrap_self(env: &Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
    user.require_auth();
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if storage::get_profile(env, &user).is_none() {
        let initial = storage::get_default_bootstrap_credits(env);
        let profile = Profile::new(env.ledger().timestamp(), initial);
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped {
            user,
            initial_credits: initial,
        }
        .publish(env);
    }

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn spend(
    env: &Env,
    user: Address,
    amount: u32,
    reason: Symbol,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if amount == 0 {
        // No-op spend. Still mark seen so retries are idempotent.
        idempotency::mark_seen(env, &op_id);
        return Ok(());
    }

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    if profile.credits < amount {
        return Err(Error::InsufficientCredits);
    }
    profile.credits -= amount;
    storage::set_profile(env, &user, &profile);

    evt::CreditsSpent {
        user,
        amount,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn earn(
    env: &Env,
    user: Address,
    amount: u32,
    reason: Symbol,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    profile.credits = profile.credits.saturating_add(amount);
    storage::set_profile(env, &user, &profile);

    evt::CreditsEarned {
        user,
        amount,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn refund(
    env: &Env,
    user: Address,
    amount: u32,
    reason: Symbol,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut profile = storage::get_profile(env, &user).ok_or(Error::ProfileNotFound)?;
    profile.credits = profile.credits.saturating_add(amount);
    storage::set_profile(env, &user, &profile);

    evt::CreditsRefunded {
        user,
        amount,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn admin_grant(
    env: &Env,
    user: Address,
    amount: u32,
    reason: String,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_admin(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if reason.len() == 0 {
        return Err(Error::ReasonRequired);
    }

    let mut profile = storage::get_profile(env, &user).unwrap_or_else(|| {
        let initial = storage::get_default_bootstrap_credits(env);
        Profile::new(env.ledger().timestamp(), initial)
    });
    profile.credits = profile.credits.saturating_add(amount);
    storage::set_profile(env, &user, &profile);

    evt::AdminCreditsGranted {
        user,
        amount,
        reason,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}
