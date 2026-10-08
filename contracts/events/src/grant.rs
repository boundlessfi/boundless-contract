use soroban_sdk::{Address, BytesN, Env, Symbol};

use crate::admin;
use crate::errors::Error;
use crate::escrow;
use crate::event_ops::{MAX_REPUTATION_BUMP, SPLIT_BPS_TOTAL};
use crate::events as evt;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::types::{EventStatus, GrantProgress, Pillar, ReleaseKind, Winner};

struct Standing {
    position: u32,
    award: i128,
    settled: u32,
    settled_amount: i128,
}

fn standing(env: &Env, event_id: u64, recipient: &Address) -> Result<Standing, Error> {
    let roster = storage::get_grant_roster(env, event_id).ok_or(Error::NoSubmissions)?;
    let award = roster.get(recipient.clone()).ok_or(Error::NoSubmissions)?;
    let progress = storage::get_grant_progress(env, event_id, recipient);
    Ok(Standing {
        position: award.position,
        award: award.amount,
        settled: progress.settled,
        settled_amount: progress.settled_amount,
    })
}

/// One milestone's part of an award, rounded down: an even share for Multi,
/// its basis points for Split.
pub(crate) fn milestone_share(
    kind: &ReleaseKind,
    award: i128,
    milestone: u32,
) -> Result<i128, Error> {
    match kind {
        ReleaseKind::Multi(n) if *n > 0 => Ok(award / (*n as i128)),
        ReleaseKind::Split(shares) => {
            let bps = shares.get(milestone).ok_or(Error::InvalidMilestone)?;
            award
                .checked_mul(bps as i128)
                .map(|v| v / SPLIT_BPS_TOTAL as i128)
                .ok_or(Error::InvalidDistribution)
        }
        _ => Err(Error::InvalidReleaseKind),
    }
}

/// Whether every milestone of `award` pays something, so it can be released
/// in full.
pub(crate) fn every_milestone_pays(kind: &ReleaseKind, award: i128) -> bool {
    match kind {
        // Multi has no length cap, so its check stays a comparison.
        ReleaseKind::Multi(n) => *n > 0 && award >= *n as i128,
        ReleaseKind::Split(shares) => {
            (0..shares.len()).all(|m| matches!(milestone_share(kind, award, m), Ok(a) if a > 0))
        }
        ReleaseKind::Single => false,
    }
}

/// The milestone's share, or what is left of the award when this is the last
/// milestone the recipient has open. The remainder holds whichever order the
/// milestones settle in: it is this milestone's share plus every rounding.
fn share(
    standing: &Standing,
    kind: &ReleaseKind,
    total_milestones: u32,
    milestone: u32,
) -> Result<i128, Error> {
    if standing.settled.saturating_add(1) == total_milestones {
        standing
            .award
            .checked_sub(standing.settled_amount)
            .ok_or(Error::InvalidDistribution)
    } else {
        milestone_share(kind, standing.award, milestone)
    }
}

fn record_settlement(
    env: &Env,
    event_id: u64,
    recipient: &Address,
    standing: &Standing,
    amount: i128,
) {
    storage::set_grant_progress(
        env,
        event_id,
        recipient,
        &GrantProgress {
            settled: standing.settled.saturating_add(1),
            settled_amount: standing.settled_amount.saturating_add(amount),
        },
    );
}

fn release_owed(env: &Env, event_id: u64, amount: i128) -> Result<(), Error> {
    let owed = storage::owed_total(env, event_id);
    let owed_after = owed.checked_sub(amount).ok_or(Error::InsufficientEscrow)?;
    if owed_after < 0 {
        return Err(Error::InsufficientEscrow);
    }
    storage::set_owed_total(env, event_id, owed_after);
    Ok(())
}

// ============================================================
// CLAIM MILESTONE
// ============================================================
pub fn claim_milestone(
    env: &Env,
    event_id: u64,
    recipient: Address,
    milestone: u32,
    reputation_bump: u32,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    if reputation_bump > MAX_REPUTATION_BUMP {
        return Err(Error::ReputationBumpTooLarge);
    }

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }

    let total_milestones = match event.release_kind.milestones() {
        Some(n) if n > 0 => n,
        _ => return Err(Error::InvalidReleaseKind),
    };

    if milestone >= total_milestones {
        return Err(Error::InvalidMilestone);
    }

    event.owner.require_auth();
    idempotency::require_unseen(env, &event.owner, &op_id)?;

    let is_crowdfunding = matches!(event.pillar, Pillar::Crowdfunding);
    if is_crowdfunding {
        admin::release_cosigner(env)?.require_auth();
    }

    if storage::is_milestone_claimed(env, event_id, &recipient, milestone) {
        return Err(Error::MilestoneAlreadyClaimed);
    }

    // A campaign's only recipient is its owner, anchored at position 1 when the
    // campaign was created.
    let standing = if is_crowdfunding {
        if recipient != event.owner {
            return Err(Error::NoSubmissions);
        }
        None
    } else {
        Some(standing(env, event_id, &recipient)?)
    };

    let amount: i128 = if let Some(standing) = &standing {
        // The award carries its own amount, set at selection. Milestones split
        // that, and whichever settles last takes the rounding remainder.
        share(standing, &event.release_kind, total_milestones, milestone)?
    } else {
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
    };
    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }
    if amount > event.remaining_escrow {
        return Err(Error::InsufficientEscrow);
    }

    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);
    storage::mark_milestone_claimed(env, event_id, &recipient, milestone);
    // Crowdfunding never reserves, since it has no winner selection.
    if let Some(standing) = &standing {
        release_owed(env, event_id, amount)?;
        record_settlement(env, event_id, &recipient, standing, amount);
    } else {
        let claimed = storage::get_crowdfunding_milestones_claimed(env, event_id);
        storage::set_crowdfunding_milestones_claimed(env, event_id, claimed.saturating_add(1));
    }
    storage::append_winner(
        env,
        event_id,
        &Winner {
            recipient: recipient.clone(),
            position: standing.as_ref().map_or(1, |s| s.position),
            amount,
            milestone: Some(milestone),
            paid_at: Some(env.ledger().timestamp()),
        },
    );
    if event.remaining_escrow == 0 {
        event.status = EventStatus::Completed;
    }
    storage::set_event(env, event_id, &event);
    idempotency::mark_seen(env, &event.owner, &op_id);

    // State written above; release last so a reentrant token can't double-claim.
    let fee = if is_crowdfunding {
        let bps = escrow::effective_fee_bps(env, event.fee_bps_override);
        escrow::release_with_fee_at(env, &event.token, &recipient, amount, bps);
        escrow::compute_fee_at(amount, bps)
    } else {
        escrow::release(env, &event.token, &recipient, amount);
        0
    };

    evt::MilestoneClaimed {
        event_id,
        recipient: recipient.clone(),
        milestone,
        amount,
    }
    .publish(env);
    if fee > 0 {
        evt::MilestoneFeeCharged {
            event_id,
            recipient: recipient.clone(),
            milestone,
            fee,
        }
        .publish(env);
    }

    // Best-effort: the payout is final, so a profile failure must not revert it.
    let profile = profile_client::client(env);
    let reason = Symbol::new(env, "milestone");

    let bootstrap_op = idempotency::derive_child(env, &event.owner, &op_id, tag::BOOTSTRAP);
    let _ = profile.try_bootstrap(&recipient, &bootstrap_op);

    let rep_op = idempotency::derive_child(env, &event.owner, &op_id, tag::BUMP_REP);
    let _ = profile.try_bump_reputation(&recipient, &reputation_bump, &reason, &rep_op);

    let earnings_op = idempotency::derive_child(env, &event.owner, &op_id, tag::REGISTER_EARNINGS);
    let _ = profile.try_register_earnings(&recipient, &event.token, &amount, &earnings_op);

    Ok(())
}

// ============================================================
// FORFEIT MILESTONE
// ============================================================
/// Ends a milestone without paying it. Its share stays in escrow, no longer
/// owed to anyone, and goes back with the rest when the grant is closed.
pub fn forfeit_milestone(
    env: &Env,
    event_id: u64,
    recipient: Address,
    milestone: u32,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if !matches!(event.pillar, Pillar::Grant) {
        return Err(Error::InvalidPillar);
    }
    let total_milestones = match event.release_kind.milestones() {
        Some(n) if n > 0 => n,
        _ => return Err(Error::InvalidReleaseKind),
    };
    if milestone >= total_milestones {
        return Err(Error::InvalidMilestone);
    }

    event.owner.require_auth();
    idempotency::require_unseen(env, &event.owner, &op_id)?;

    if storage::is_milestone_claimed(env, event_id, &recipient, milestone) {
        return Err(Error::MilestoneAlreadyClaimed);
    }

    let standing = standing(env, event_id, &recipient)?;
    let amount = share(&standing, &event.release_kind, total_milestones, milestone)?;
    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }

    release_owed(env, event_id, amount)?;
    storage::mark_milestone_claimed(env, event_id, &recipient, milestone);
    record_settlement(env, event_id, &recipient, &standing, amount);
    idempotency::mark_seen(env, &event.owner, &op_id);

    evt::MilestoneForfeited {
        event_id,
        recipient,
        milestone,
        amount,
    }
    .publish(env);
    Ok(())
}
