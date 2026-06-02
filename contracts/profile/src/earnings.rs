// boundless-profile: per-token earnings registration.
//
// Spec: boundless-credits-reputation-prd.md Section 5.4.

use soroban_sdk::{Address, BytesN, Env};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency;
use crate::storage;

pub fn register(
    env: &Env,
    user: Address,
    token: Address,
    amount: i128,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_events_contract(env)?;
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    let current = storage::get_earnings(env, &user, &token);
    let new = current.saturating_add(amount);
    storage::set_earnings(env, &user, &token, new);

    evt::EarningsRegistered {
        user,
        token,
        amount,
    }
    .publish(env);
    idempotency::mark_seen(env, &op_id);
    Ok(())
}
