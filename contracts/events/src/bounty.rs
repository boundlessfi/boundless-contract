use soroban_sdk::{Address, BytesN, Env};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::types::{EventRecord, EventStatus, Pillar, ReleaseKind};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    if !matches!(record.release_kind, ReleaseKind::Single) {
        return Err(Error::InvalidReleaseKind);
    }
    Ok(())
}

// ============================================================
// APPLY
// ============================================================
pub fn apply(
    env: &Env,
    bounty_id: u64,
    applicant: Address,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;

    let event = storage::get_event(env, bounty_id).ok_or(Error::EventNotFound)?;
    require_active_bounty(&event)?;

    applicant.require_auth();
    idempotency::require_unseen(env, &applicant, &op_id)?;

    storage::append_applicant(env, bounty_id, &applicant)?;

    let profile = profile_client::client(env);
    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    profile.bootstrap(&applicant, &bootstrap_op);

    evt::Applied {
        event_id: bounty_id,
        applicant: applicant.clone(),
    }
    .publish(env);

    idempotency::mark_seen(env, &applicant, &op_id);
    Ok(())
}

// ============================================================
// WITHDRAW APPLICATION
// ============================================================
pub fn withdraw_application(
    env: &Env,
    bounty_id: u64,
    applicant: Address,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;

    let event = storage::get_event(env, bounty_id).ok_or(Error::EventNotFound)?;
    require_active_bounty(&event)?;

    applicant.require_auth();
    idempotency::require_unseen(env, &applicant, &op_id)?;

    if storage::has_any_submission(env, bounty_id, &applicant) {
        return Err(Error::SubmissionAlreadyExists);
    }

    storage::remove_applicant(env, bounty_id, &applicant)?;

    evt::ApplicationWithdrawn {
        event_id: bounty_id,
        applicant: applicant.clone(),
    }
    .publish(env);

    idempotency::mark_seen(env, &applicant, &op_id);
    Ok(())
}

// ============================================================
// HELPERS
// ============================================================
fn require_active_bounty(event: &EventRecord) -> Result<(), Error> {
    if !matches!(event.pillar, Pillar::Bounty) {
        return Err(Error::InvalidPillar);
    }
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    Ok(())
}
