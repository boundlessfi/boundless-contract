use soroban_sdk::{Address, BytesN, Env, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::escrow;
use crate::events as evt;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::types::{EventRecord, EventStatus, Pillar, ReleaseKind, Winner};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    match record.release_kind {
        ReleaseKind::Multi(n) if n > 0 => Ok(()),
        _ => Err(Error::InvalidReleaseKind),
    }
}

// ============================================================
// CLAIM MILESTONE
// ============================================================
pub fn claim_milestone(
    env: &Env,
    event_id: u64,
    recipient: Address,
    milestone: u32,
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

    if matches!(event.pillar, Pillar::Crowdfunding) {
        let admin = storage::get_admin(env)?;
        admin.require_auth();
    }

    if storage::is_milestone_claimed(env, event_id, &recipient, milestone) {
        return Err(Error::MilestoneAlreadyClaimed);
    }

    let is_crowdfunding = matches!(event.pillar, Pillar::Crowdfunding);

    let anchor_idx =
        storage::get_grant_recipient_idx(env, event_id, &recipient).ok_or(Error::NoSubmissions)?;
    let anchor = storage::winner_at(env, event_id, anchor_idx).ok_or(Error::NoSubmissions)?;
    if anchor.recipient != recipient || anchor.milestone.is_some() {
        return Err(Error::NoSubmissions);
    }
    let position = anchor.position;
    let reputation_bump = anchor.reputation_bump.unwrap_or(0);
    let already_claimed_for_recipient =
        storage::get_grant_recipient_claim_count(env, event_id, &recipient);
    let already_paid_to_recipient = if is_crowdfunding {
        0
    } else {
        let percent = event.winner_distribution.get(position).unwrap_or(0) as i128;
        let total_share = event.total_budget.saturating_mul(percent) / 100_i128;
        let per_milestone_floored = total_share / (total_milestones as i128);
        (already_claimed_for_recipient as i128).saturating_mul(per_milestone_floored)
    };

    let amount: i128 = if is_crowdfunding {
        let claimed_count = storage::get_crowdfunding_milestones_claimed(env, event_id);
        let remaining_milestones = total_milestones.saturating_sub(claimed_count);
        if remaining_milestones == 0 {
            return Err(Error::InvalidMilestone);
        }
        if event.remaining_escrow <= 0 {
            return Err(Error::InsufficientEscrow);
        }
        if remaining_milestones == 1 {
            event.remaining_escrow
        } else {
            event.remaining_escrow / (remaining_milestones as i128)
        }
    } else {
        let percent = event
            .winner_distribution
            .get(position)
            .ok_or(Error::InvalidWinnerPosition)? as i128;
        let total_share = event.total_budget.saturating_mul(percent) / 100_i128;
        let per_milestone_floored = total_share / (total_milestones as i128);

        if already_claimed_for_recipient.saturating_add(1) == total_milestones {
            total_share.saturating_sub(already_paid_to_recipient)
        } else {
            per_milestone_floored
        }
    };
    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }
    if amount > event.remaining_escrow {
        return Err(Error::InsufficientEscrow);
    }

    if is_crowdfunding {
        let bps = escrow::effective_fee_bps(env, event.fee_bps_override);
        escrow::release_with_fee_at(env, &event.token, &recipient, amount, bps);
    } else {
        escrow::release(env, &event.token, &recipient, amount);
    }
    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);
    storage::mark_milestone_claimed(env, event_id, &recipient, milestone);
    storage::increment_grant_recipient_claim_count(env, event_id, &recipient);
    if is_crowdfunding {
        let claimed = storage::get_crowdfunding_milestones_claimed(env, event_id);
        storage::set_crowdfunding_milestones_claimed(env, event_id, claimed.saturating_add(1));
    }

    storage::append_winner(
        env,
        event_id,
        &Winner {
            recipient: recipient.clone(),
            position,
            amount,
            milestone: Some(milestone),
            paid_at: Some(env.ledger().timestamp()),
            reputation_bump: Some(reputation_bump),
        },
    );

    if event.remaining_escrow == 0 {
        event.status = EventStatus::Completed;
    }
    storage::set_event(env, event_id, &event);

    evt::MilestoneClaimed {
        event_id,
        recipient: recipient.clone(),
        milestone,
        amount,
    }
    .publish(env);

    // Best-effort profile side effects — payout is already finalised so a
    // profile-contract failure must not block the claim.
    let profile = profile_client::client(env);
    let reason = Symbol::new(env, "milestone");

    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    let _ = profile.try_bootstrap(&recipient, &bootstrap_op);

    let rep_op = idempotency::derive_child(env, &op_id, tag::BUMP_REP);
    let _ = profile.try_bump_reputation(&recipient, &reputation_bump, &reason, &rep_op);

    let earnings_op = idempotency::derive_child(env, &op_id, tag::REGISTER_EARNINGS);
    let _ = profile.try_register_earnings(&recipient, &event.token, &amount, &earnings_op);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}
