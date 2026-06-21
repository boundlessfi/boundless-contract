// boundless-events: grant-specific behavior + shared milestone-claim entry.
//
// Spec: boundless-platform-contract-prd.md Sections 6.4, 7;
//       boundless-crowdfunding-prd.md.
//
// Grants use ReleaseKind::Multi(n) and release per-milestone via
// claim_milestone. Crowdfunding reuses the same entry point but takes a
// different math path (dynamic, see comment in claim_milestone).

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
//
// Per-(recipient, milestone) idempotency via DataKey::MilestoneClaimed.
//
// Math depends on pillar:
//
//   Grant (fixed):
//     amount = total_budget * distribution[position] / 100 / total_milestones
//
//   Crowdfunding (dynamic):
//     amount = remaining_escrow / (total_milestones - already_claimed_count)
//
//   The Crowdfunding path divides whatever escrow is actually present at
//   release time evenly across the remaining milestones. The last milestone
//   picks up any rounding remainder, so the total paid equals what was
//   raised exactly.
//
// Each call: token release + bootstrap (idempotent) + earn_credits +
// bump_reputation + register_earnings. Marks the event Completed when
// the last milestone for the last recipient drains remaining_escrow.
//
// Auth: event.owner for grants (organization-controlled); for crowdfunding
// the owner is the builder, so the off-chain layer routes claim_milestone
// through an admin-signed transaction where the admin signs on the builder's
// behalf only after milestone validation. On-chain check is identical:
// require_auth on event.owner. Operationally that means the builder's
// abstracted-wallet key is used to sign, gated by admin approval upstream.
//
// Spec: boundless-platform-contract-prd.md Section 6.4, 8;
//       boundless-crowdfunding-prd.md.
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

    // M5 (2026-06 audit): crowdfunding claims require admin co-sign on top
    // of the builder's auth. Today the abstracted-wallet model keeps the
    // builder's key on the platform side, so this is operationally a
    // no-op; the on-chain check is the defense if the wallet model ever
    // changes to put the key in the builder's hands. Grants stay
    // single-auth because the event owner is the grant org, which is the
    // intended approver. See docs/audit-2026-06-stellar-skill.md M5.
    if matches!(event.pillar, Pillar::Crowdfunding) {
        let admin = storage::get_admin(env)?;
        admin.require_auth();
    }

    if storage::is_milestone_claimed(env, event_id, &recipient, milestone) {
        return Err(Error::MilestoneAlreadyClaimed);
    }

    // Locate the recipient's anchor row (milestone == None), and count any
    // prior per-milestone payouts so the fixed-split last-milestone sweep
    // can land exactly on total_share.
    let count = storage::winner_count(env, event_id);
    let mut winner_position: Option<u32> = None;
    let mut already_claimed_for_recipient: u32 = 0;
    let mut already_paid_to_recipient: i128 = 0;
    for idx in 0..count {
        let w = match storage::winner_at(env, event_id, idx) {
            Some(w) => w,
            None => continue,
        };
        if w.recipient != recipient {
            continue;
        }
        match w.milestone {
            None => winner_position = Some(w.position),
            Some(_) => {
                already_claimed_for_recipient = already_claimed_for_recipient.saturating_add(1);
                already_paid_to_recipient = already_paid_to_recipient.saturating_add(w.amount);
            }
        }
    }
    let position = winner_position.ok_or(Error::NoSubmissions)?;

    let is_crowdfunding = matches!(event.pillar, Pillar::Crowdfunding);
    let amount: i128 = if is_crowdfunding {
        // Dynamic split: divide what's left evenly across remaining milestones;
        // the final claim drains the remainder so no dust is stranded.
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
        // Fixed split with last-milestone sweep so the position share pays out
        // exactly. Per-recipient progress was tallied during the anchor scan
        // above.
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

    // Move money.
    escrow::release(env, &event.token, &recipient, amount);
    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);
    storage::mark_milestone_claimed(env, event_id, &recipient, milestone);
    if is_crowdfunding {
        let claimed = storage::get_crowdfunding_milestones_claimed(env, event_id);
        storage::set_crowdfunding_milestones_claimed(env, event_id, claimed.saturating_add(1));
    }

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

    // Append per-milestone Winner row for the audit trail.
    storage::append_winner(
        env,
        event_id,
        &Winner {
            recipient: recipient.clone(),
            position,
            amount,
            milestone: Some(milestone),
            paid_at: Some(env.ledger().timestamp()),
        },
    );

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
