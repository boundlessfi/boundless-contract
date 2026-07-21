use soroban_sdk::{Address, BytesN, Env};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency;
use crate::storage;
use crate::types::Profile;

pub fn bootstrap(env: &Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    let domain = idempotency::events_domain(env)?;
    idempotency::require_unseen(env, &domain, &op_id)?;

    if storage::get_profile(env, &user).is_none() {
        let profile = Profile::new(env.ledger().timestamp());
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped { user }.publish(env);
    }

    idempotency::mark_seen(env, &domain, &op_id);
    Ok(())
}

pub fn bootstrap_self(env: &Env, user: Address, op_id: BytesN<32>) -> Result<(), Error> {
    user.require_auth();
    admin::require_not_paused(env)?;
    // Domain = user so unprivileged self-bootstrap cannot squat events-domain op_ids.
    idempotency::require_unseen(env, &user, &op_id)?;

    if storage::get_profile(env, &user).is_none() {
        let profile = Profile::new(env.ledger().timestamp());
        storage::set_profile(env, &user, &profile);
        evt::ProfileBootstrapped { user: user.clone() }.publish(env);
    }

    idempotency::mark_seen(env, &user, &op_id);
    Ok(())
}
