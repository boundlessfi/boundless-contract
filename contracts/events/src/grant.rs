// boundless-events: grant-specific behavior.
//
// Spec: boundless-platform-contract-prd.md Sections 6.4, 7.
//
// Grants use ReleaseKind::Multi(n) and release per-milestone via
// claim_milestone.

use soroban_sdk::{Address, BytesN, Env, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::escrow;
use crate::events as evt;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::types::{EventRecord, EventStatus, ReleaseKind, Winner};

pub fn validate_create(
    _env: &Env,
    record: &EventRecord,
    _owner: &Address,
) -> Result<(), Error> {
    match record.release_kind {
        ReleaseKind::Multi(n) if n > 0 => Ok(()),
        _ => Err(Error::InvalidReleaseKind),
    }
}

// ============================================================
// CLAIM MILESTONE
// ============================================================
//
// Per-(recipient, milestone) idempotency via DataKey::MilestoneClaimed.
// Amount per milestone for a recipient at position p:
//
//   amount = total_budget * distribution[p] / 100 / total_milestones
//
// Each call: token release + bootstrap (idempotent) + earn_credits +
// bump_reputation + register_earnings. Marks the event Completed when the
// last milestone for the last recipient drains remaining_escrow.
//
// Spec: boundless-platform-contract-prd.md Section 6.4, 8.
pub fn claim_milestone(
    env: &Env,
    event_id: u64,
    recipient: Address,
    milestone: u32,
    credit_earn: u32,
    reputation_bump: u32,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }

    let total_milestones = match event.release_kind {
        ReleaseKind::Multi(n) if n > 0 => n,
        _ => return Err(Error::InvalidReleaseKind),
    };

    if milestone >= total_milestones {
        return Err(Error::InvalidMilestone);
    }

    event.owner.require_auth();

    if storage::is_milestone_claimed(env, event_id, &recipient, milestone) {
        return Err(Error::MilestoneAlreadyClaimed);
    }

    // Locate the recipient in the recorded winners (the milestone=None entry
    // written by select_winners). Without that anchor there is nothing to pay.
    let winners = storage::get_winners(env, event_id);
    let mut winner_position: Option<u32> = None;
    for w in winners.iter() {
        if w.recipient == recipient && w.milestone.is_none() {
            winner_position = Some(w.position);
            break;
        }
    }
    let position = winner_position.ok_or(Error::NoSubmissions)?;

    let percent = event
        .winner_distribution
        .get(position)
        .ok_or(Error::InvalidWinnerPosition)? as i128;
    let total_share = event.total_budget.saturating_mul(percent) / 100_i128;
    let amount = total_share / (total_milestones as i128);
    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }
    if amount > event.remaining_escrow {
        return Err(Error::InsufficientEscrow);
    }

    // Move money.
    escrow::release(env, &event.token, &recipient, amount);
    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);
    storage::mark_milestone_claimed(env, event_id, &recipient, milestone);

    // Cross-contract profile mutations. Each call gets a unique child op_id.
    let profile = profile_client::client(env);
    let reason = Symbol::new(env, "milestone");

    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    profile.bootstrap(&recipient, &bootstrap_op);

    let earn_op = idempotency::derive_child(env, &op_id, tag::EARN_CREDITS);
    profile.earn_credits(&recipient, &credit_earn, &reason, &earn_op);

    let rep_op = idempotency::derive_child(env, &op_id, tag::BUMP_REP);
    profile.bump_reputation(&recipient, &reputation_bump, &reason, &rep_op);

    let earnings_op = idempotency::derive_child(env, &op_id, tag::REGISTER_EARNINGS);
    profile.register_earnings(&recipient, &event.token, &amount, &earnings_op);

    // Append per-milestone Winner record (audit trail).
    let mut updated_winners = winners;
    updated_winners.push_back(Winner {
        recipient: recipient.clone(),
        position,
        amount,
        milestone: Some(milestone),
        paid_at: Some(env.ledger().timestamp()),
    });
    storage::set_winners(env, event_id, &updated_winners);

    if event.remaining_escrow == 0 {
        event.status = EventStatus::Completed;
    }
    storage::set_event(env, event_id, &event);

    evt::MilestoneClaimed {
        event_id,
        recipient,
        milestone,
        amount,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}
