// boundless-profile: profile lifecycle (bootstrap).
//
// Credits were removed (2026-06) and are now an off-chain ledger. Bootstrap
// only creates the per-user profile so reputation and earnings can attach.

use soroban_sdk::{Address, BytesN, Env};

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
        let profile = Profile::new(env.ledger().timestamp());
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped { user }.publish(env);
    }

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

/// Self-service bootstrap. A user creates their OWN profile by authorizing the
/// call with their wallet (no admin key, no events-contract dependency). Used
/// at onboarding so every user has a profile before they participate.
/// Idempotent: a second call when the profile already exists is a no-op.
pub fn bootstrap_self(env: &Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
    user.require_auth();
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if storage::get_profile(env, &user).is_none() {
        let profile = Profile::new(env.ledger().timestamp());
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped { user }.publish(env);
    }

    idempotency::mark_seen(env, &op_id);
    Ok(())
}
