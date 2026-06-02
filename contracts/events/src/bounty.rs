// boundless-events: bounty-specific behavior.
//
// Spec: boundless-platform-contract-prd.md Sections 6.3, 7.
//
// Bounties use ReleaseKind::Single. Apply / submit are gated by credits.

use soroban_sdk::{Address, BytesN, Env, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::types::{EventRecord, EventStatus, Pillar, ReleaseKind};

pub fn validate_create(
    _env: &Env,
    record: &EventRecord,
    _owner: &Address,
) -> Result<(), Error> {
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

    // Reject duplicate application.
    let mut applicants = storage::get_applicants(env, bounty_id);
    for existing in applicants.iter() {
        if existing == applicant {
            return Err(Error::ApplicantAlreadyApplied);
        }
    }

    // Cross-contract: bootstrap (idempotent), then spend credits.
    let profile = profile_client::client(env);
    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    profile.bootstrap(&applicant, &bootstrap_op);

    let spend_op = idempotency::derive_child(env, &op_id, tag::SPEND_CREDITS);
    profile.spend_credits(
        &applicant,
        &event.application_credit_cost,
        &Symbol::new(env, "apply"),
        &spend_op,
    );

    applicants.push_back(applicant.clone());
    storage::set_applicants(env, bounty_id, &applicants);

    evt::Applied {
        event_id: bounty_id,
        applicant,
        credit_cost: event.application_credit_cost,
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

    // Locate and remove the applicant.
    let mut applicants = storage::get_applicants(env, bounty_id);
    let mut found_at: Option<u32> = None;
    for (idx, existing) in applicants.iter().enumerate() {
        if existing == applicant {
            found_at = Some(idx as u32);
            break;
        }
    }
    let idx = found_at.ok_or(Error::ApplicantNotApplied)?;
    applicants.remove(idx);
    storage::set_applicants(env, bounty_id, &applicants);

    // Cross-contract: refund 50% of the application credit cost.
    let refund = event.application_credit_cost / 2;
    if refund > 0 {
        let profile = profile_client::client(env);
        let refund_op = idempotency::derive_child(env, &op_id, tag::REFUND_CREDITS);
        profile.refund_credits(
            &applicant,
            &refund,
            &Symbol::new(env, "wd_refund"),
            &refund_op,
        );
    }

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
