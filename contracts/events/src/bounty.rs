// boundless-events: bounty-specific behavior.
//
// Spec: boundless-platform-contract-prd.md Sections 6.3, 7.
//
// Bounties use ReleaseKind::Single. Credits (apply cost / refunds) are handled
// off-chain; the contract only records applicants and ensures their profile.

use soroban_sdk::{Address, BytesN, Env};

use crate::admin;
use crate::errors::Error;
use crate::event_ops::MAX_APPLICANTS_PER_EVENT;
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
    idempotency::require_unseen(env, &op_id)?;

    let event = storage::get_event(env, bounty_id).ok_or(Error::EventNotFound)?;
    require_active_bounty(env, &event)?;

    applicant.require_auth();

    // append_applicant returns Err on duplicate or cap exceeded.
    storage::append_applicant(env, bounty_id, &applicant, MAX_APPLICANTS_PER_EVENT)?;

    // Cross-contract: ensure the applicant has a profile (idempotent). Credits
    // are charged off-chain now, so there is no on-chain spend here.
    let profile = profile_client::client(env);
    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    profile.bootstrap(&applicant, &bootstrap_op);

    evt::Applied {
        event_id: bounty_id,
        applicant,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
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
    idempotency::require_unseen(env, &op_id)?;

    let event = storage::get_event(env, bounty_id).ok_or(Error::EventNotFound)?;
    require_active_bounty(env, &event)?;

    applicant.require_auth();

    // Reject withdrawal if the applicant already submitted.
    if storage::get_submission(env, bounty_id, &applicant).is_some() {
        return Err(Error::SubmissionAlreadyExists);
    }

    // Membership check + swap-remove. Slot lookup is O(1), so this avoids
    // the prior O(n) linear scan even at the cap.
    storage::remove_applicant(env, bounty_id, &applicant)?;

    // Credits (including any withdrawal refund) are handled off-chain.

    evt::ApplicationWithdrawn {
        event_id: bounty_id,
        applicant,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// HELPERS
// ============================================================
fn require_active_bounty(env: &Env, event: &EventRecord) -> Result<(), Error> {
    if !matches!(event.pillar, Pillar::Bounty) {
        return Err(Error::InvalidPillar);
    }
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if let Some(deadline) = event.deadline {
        if deadline <= env.ledger().timestamp() {
            return Err(Error::DeadlinePassed);
        }
    }
    Ok(())
}
