// boundless-events: canonical event lifecycle operations.
//
// Spec: boundless-platform-contract-prd.md Sections 6.2, 6.4, 6.5, 7.

use soroban_sdk::{Address, BytesN, Env, String, Symbol, Vec};

use crate::admin::{self, MAX_FEE_BPS};
use crate::bounty;
use crate::crowdfunding;
use crate::errors::Error;
use crate::escrow;
use crate::events as evt;
use crate::grant;
use crate::hackathon;
use crate::idempotency::{self, tag};
use crate::profile_client;
use crate::storage;
use crate::token_whitelist;
use crate::types::{
    CancellationBranch, CancellationState, CreateEventParams, EventRecord, EventStatus, Pillar,
    ReleaseKind, Submission, Winner, WinnerSpec,
};

const MAX_TITLE_LEN: u32 = 120;
const MAX_APPLY_COST: u32 = 100;
const MAX_WINNERS_PER_SELECT: u32 = 50;

// Per-event list caps. Lifted from 100 to 5_000 once paged cancel landed:
// start_cancel / process_cancel_batch / finalize_cancel split the refund
// pass across multiple txs so the per-tx footprint never blows past
// MAX_REFUNDS_PER_BATCH contributors.
//
// Spec: docs/audit-2026-06-stellar-skill.md, H3/H4 + paged-cancel follow-up.
pub const MAX_APPLICANTS_PER_EVENT: u32 = 5_000;
pub const MAX_CONTRIBUTORS_PER_EVENT: u32 = 5_000;

// Max refunds per process_cancel_batch tx. Conservative; the actual ceiling
// depends on the token's transfer cost. 25 is well below Soroban's ~100-entry
// write footprint when each refund touches ContributorAmount + a token
// transfer (3-4 ledger entries).
pub const MAX_REFUNDS_PER_BATCH: u32 = 25;

// Open-contribution floor: 10 USDC at 7 decimals. The check is denominated in
// stroops because every supported token on the whitelist uses Stellar's
// canonical 7-decimal scale. If a future token adopts a different scale the
// whitelist registration is the place to gate it; the contract floor stays
// uniform.
//
// Spec: boundless-partner-contributions-prd.md Section 6.1.
const MIN_CONTRIBUTION_STROOPS: i128 = 100_000_000_i128; // 10 * 10^7

// ============================================================
// CREATE EVENT
// ============================================================
/// The address authorized to manage an event (select winners, cancel). A
/// per-event manager override takes precedence; otherwise management falls back
/// to the event owner (legacy events created before manager support). This
/// decouples the funding source (owner) from the operating identity (manager).
fn resolve_manager(env: &Env, event_id: u64, owner: &Address) -> Address {
    storage::get_event_manager(env, event_id).unwrap_or_else(|| owner.clone())
}

pub fn create_event(
    env: &Env,
    params: CreateEventParams,
    op_id: BytesN<32>,
) -> Result<u64, Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    params.owner.require_auth();

    // Token whitelist.
    token_whitelist::require_supported(env, &params.token)?;

    // Title length.
    if params.title.len() > MAX_TITLE_LEN {
        return Err(Error::TitleTooLong);
    }

    // Budget.
    if params.total_budget <= 0 {
        return Err(Error::InvalidBudget);
    }

    // Distribution: at least one entry, percents sum to 100.
    if params.winner_distribution.is_empty() {
        return Err(Error::InvalidDistribution);
    }
    let mut sum: u32 = 0;
    for (_pos, percent) in params.winner_distribution.iter() {
        sum = sum.saturating_add(percent);
    }
    if sum != 100 {
        return Err(Error::DistributionMismatch);
    }

    // Deadline (if set) must be future.
    if let Some(deadline) = params.deadline {
        if deadline <= env.ledger().timestamp() {
            return Err(Error::DeadlineMustBeFuture);
        }
    }

    // Application credit cost cap.
    if params.application_credit_cost > MAX_APPLY_COST {
        return Err(Error::InvalidPillar);
    }

    if let Some(bps) = params.fee_bps_override {
        if bps > MAX_FEE_BPS {
            return Err(Error::InvalidFeeBps);
        }
    }
    let effective_bps = escrow::effective_fee_bps(env, params.fee_bps_override);

    // Crowdfunding flips total_budget into a funding goal; escrow starts at 0
    // and grows only via add_funds. Every other pillar deposits at create.
    let is_crowdfunding = matches!(params.pillar, Pillar::Crowdfunding);
    let initial_escrow: i128 = if is_crowdfunding { 0 } else { params.total_budget };

    let provisional = EventRecord {
        id: 0,
        pillar: params.pillar.clone(),
        owner: params.owner.clone(),
        token: params.token.clone(),
        total_budget: params.total_budget,
        remaining_escrow: initial_escrow,
        release_kind: params.release_kind.clone(),
        status: EventStatus::Active,
        content_uri: params.content_uri.clone(),
        title: params.title.clone(),
        created_at: env.ledger().timestamp(),
        deadline: params.deadline,
        winner_distribution: params.winner_distribution.clone(),
        application_credit_cost: params.application_credit_cost,
        fee_bps_override: params.fee_bps_override,
    };
    match params.pillar {
        Pillar::Hackathon => hackathon::validate_create(env, &provisional, &params.owner)?,
        Pillar::Bounty => bounty::validate_create(env, &provisional, &params.owner)?,
        Pillar::Grant => grant::validate_create(env, &provisional, &params.owner)?,
        Pillar::Crowdfunding => crowdfunding::validate_create(env, &provisional, &params.owner)?,
    }

    if !is_crowdfunding {
        escrow::deposit_with_fee_at(
            env,
            &params.token,
            &params.owner,
            params.total_budget,
            effective_bps,
        );
    }

    // Assign id and persist.
    let id = idempotency::next_event_id(env);
    let record = EventRecord {
        id,
        ..provisional
    };
    storage::set_event(env, id, &record);

    // Record the management authority override when the owner delegates it (so
    // an org can fund from any wallet but keep management on its own wallet).
    if let Some(manager) = &params.manager {
        storage::set_event_manager(env, id, manager);
    }

    // Crowdfunding: pre-seat the builder as the sole winner at position 1.
    if is_crowdfunding {
        storage::append_winner(
            env,
            id,
            &Winner {
                recipient: params.owner.clone(),
                position: 1,
                amount: 0,
                milestone: None,
                paid_at: None,
            },
        );
    }

    evt::EventCreated {
        id,
        pillar: record.pillar.clone(),
        owner: record.owner.clone(),
        token: record.token.clone(),
        total_budget: record.total_budget,
        content_uri: record.content_uri.clone(),
        title: record.title.clone(),
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(id)
}

/// Re-assign (or set) the management authority for an event. Gated by the
/// current manager (the override if present, else the owner), so an org can
/// rotate its operating wallet but an outsider cannot hijack management.
pub fn set_manager(env: &Env, event_id: u64, new_manager: Address) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    resolve_manager(env, event_id, &event.owner).require_auth();
    storage::set_event_manager(env, event_id, &new_manager);
    Ok(())
}

/// The current management authority for an event (override if set, else owner).
pub fn get_manager(env: &Env, event_id: u64) -> Result<Address, Error> {
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(resolve_manager(env, event_id, &event.owner))
}

// ============================================================
// ADD FUNDS (partner / community contribution)
// ============================================================
pub fn add_funds(
    env: &Env,
    event_id: u64,
    from: Address,
    amount: i128,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    if amount <= 0 {
        return Err(Error::InvalidContributionAmount);
    }
    if amount < MIN_CONTRIBUTION_STROOPS {
        return Err(Error::BelowMinimumContribution);
    }

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }

    from.require_auth();

    // First-time contributor? Reserve the slot BEFORE moving tokens so a
    // cap-exceeded reverts the whole flow with no half-credited row.
    if from != event.owner {
        let prior = storage::get_contributor_amount(env, event_id, &from);
        if prior == 0 {
            storage::append_contributor(env, event_id, &from, MAX_CONTRIBUTORS_PER_EVENT)?;
        }
    }

    // Use the rate snapshotted at publish so add_funds matches the program's
    // quoted rate even if the contract default changes mid-flight.
    let effective_bps = escrow::effective_fee_bps(env, event.fee_bps_override);
    let credited = escrow::deposit_with_fee_at(env, &event.token, &from, amount, effective_bps);
    event.remaining_escrow = event.remaining_escrow.saturating_add(credited);

    if from != event.owner {
        let prior = storage::get_contributor_amount(env, event_id, &from);
        let new_total = prior.saturating_add(credited);
        storage::set_contributor_amount(env, event_id, &from, new_total);
    }

    storage::set_event(env, event_id, &event);

    evt::FundsAdded {
        event_id,
        contributor: from,
        amount: credited,
        new_remaining: event.remaining_escrow,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// PAGED CANCEL
//
// Three-step flow to keep cancel inside Soroban's per-tx footprint budget:
//
//   1. start_cancel(id)           — flip Active → Cancelling, snapshot the
//                                   refund math (non_owner_total, remaining,
//                                   count, branch). For events with 0
//                                   contributors, also handles the owner
//                                   refund inline.
//   2. process_cancel_batch(id, n) — refund up to n contributors at the
//                                   cursor. Repeats until cursor == count.
//   3. finalize_cancel(id)         — require cursor exhausted; pay owner
//                                   residual on FullPartnerThenResidual;
//                                   flip Cancelling → Cancelled; clear the
//                                   state entry.
//
// Refund math (snapshotted at start_cancel; stable across batches because
// Cancelling status blocks add_funds + other contributor mutations):
//
//   non_owner_total = sum(ContributorAmount(event_id, *))
//   remaining       = event.remaining_escrow
//
//   OwnerOnly:               non_owner_total == 0; owner gets remaining.
//                            Settled inline at start_cancel.
//   FullPartnerThenResidual: remaining >= non_owner_total; each partner
//                            gets full amount; owner residual paid at
//                            finalize_cancel.
//   ProRataPartners:         remaining < non_owner_total; partners get
//                            floor(amt * remaining / non_owner_total).
//                            Owner gets 0. Dust stays in contract.
//
// Spec: boundless-platform-contract-prd.md Section 6.2;
//       boundless-partner-contributions-prd.md Section 7;
//       docs/audit-2026-06-stellar-skill.md paged-cancel follow-up.
// ============================================================
pub fn start_cancel(env: &Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if storage::get_cancellation_state(env, event_id).is_some() {
        return Err(Error::CancellationAlreadyStarted);
    }

    // Management authority: the per-event manager override if set, else owner.
    resolve_manager(env, event_id, &event.owner).require_auth();

    let remaining = event.remaining_escrow;
    let count = storage::contributor_count(env, event_id);

    // Sum non-owner contributions once. Bounded by MAX_CONTRIBUTORS_PER_EVENT
    // contributor_amount reads; for events with > MAX_REFUNDS_PER_BATCH
    // contributors the caller will need to start_cancel on smaller events
    // OR we accept this single read pass as the cost of snapshotting.
    let mut non_owner_total: i128 = 0;
    for idx in 0..count {
        if let Some(c) = storage::contributor_at(env, event_id, idx) {
            non_owner_total =
                non_owner_total.saturating_add(storage::get_contributor_amount(env, event_id, &c));
        }
    }

    let branch = if non_owner_total <= 0 {
        CancellationBranch::OwnerOnly
    } else if remaining >= non_owner_total {
        CancellationBranch::FullPartnerThenResidual
    } else {
        CancellationBranch::ProRataPartners
    };

    // OwnerOnly shortcut: no partner refunds to page through; flip directly
    // to Cancelled and pay owner residual inline. Saves the caller a round
    // trip for the common "abandoned, no community contributions" case.
    if matches!(branch, CancellationBranch::OwnerOnly) {
        if remaining > 0 {
            escrow::release(env, &event.token, &event.owner, remaining);
            evt::OwnerResidualRefunded {
                event_id,
                owner: event.owner.clone(),
                amount: remaining,
            }
            .publish(env);
        }
        event.remaining_escrow = 0;
        event.status = EventStatus::Cancelled;
        storage::set_event(env, event_id, &event);
        evt::EventCancelled { id: event_id }.publish(env);
        idempotency::mark_seen(env, &op_id);
        return Ok(());
    }

    // Partner refund branches: persist the cursor + branch and flip to
    // Cancelling. process_cancel_batch + finalize_cancel finish the work.
    let state = CancellationState {
        non_owner_total,
        remaining_at_start: remaining,
        count_at_start: count,
        next_idx: 0,
        branch,
    };
    storage::set_cancellation_state(env, event_id, &state);
    event.status = EventStatus::Cancelling;
    storage::set_event(env, event_id, &event);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

pub fn process_cancel_batch(
    env: &Env,
    event_id: u64,
    max_refunds: u32,
    op_id: BytesN<32>,
) -> Result<u32, Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Cancelling) {
        return Err(Error::CancellationNotStarted);
    }
    // Management authority: the per-event manager override if set, else owner.
    resolve_manager(env, event_id, &event.owner).require_auth();

    let mut state =
        storage::get_cancellation_state(env, event_id).ok_or(Error::CancellationNotStarted)?;

    // Anyone can call with a 0 batch_size if the cursor is at end; nothing to do.
    let cap = if max_refunds > MAX_REFUNDS_PER_BATCH {
        MAX_REFUNDS_PER_BATCH
    } else {
        max_refunds
    };
    let mut processed: u32 = 0;
    while processed < cap && state.next_idx < state.count_at_start {
        let idx = state.next_idx;
        state.next_idx = state.next_idx.saturating_add(1);

        let c = match storage::contributor_at(env, event_id, idx) {
            Some(c) => c,
            None => continue,
        };
        let amt = storage::get_contributor_amount(env, event_id, &c);
        if amt <= 0 {
            continue;
        }

        let payout = match state.branch {
            CancellationBranch::FullPartnerThenResidual => amt,
            CancellationBranch::ProRataPartners => {
                amt.saturating_mul(state.remaining_at_start) / state.non_owner_total
            }
            // OwnerOnly was settled inline at start_cancel.
            CancellationBranch::OwnerOnly => 0,
        };

        if payout > 0 {
            escrow::release(env, &event.token, &c, payout);
            evt::ContributorRefunded {
                event_id,
                contributor: c.clone(),
                amount: payout,
            }
            .publish(env);
        }
        storage::set_contributor_amount(env, event_id, &c, 0);
        processed = processed.saturating_add(1);
    }

    storage::set_cancellation_state(env, event_id, &state);
    let remaining_to_process = state.count_at_start.saturating_sub(state.next_idx);

    idempotency::mark_seen(env, &op_id);
    Ok(remaining_to_process)
}

pub fn finalize_cancel(env: &Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Cancelling) {
        return Err(Error::CancellationNotStarted);
    }
    // Management authority: the per-event manager override if set, else owner.
    resolve_manager(env, event_id, &event.owner).require_auth();

    let state =
        storage::get_cancellation_state(env, event_id).ok_or(Error::CancellationNotStarted)?;
    if state.next_idx < state.count_at_start {
        return Err(Error::CancellationNotFinished);
    }

    // Owner residual paid only on FullPartnerThenResidual; ProRataPartners
    // intentionally pays the owner 0.
    if matches!(state.branch, CancellationBranch::FullPartnerThenResidual) {
        let owner_residual = state
            .remaining_at_start
            .saturating_sub(state.non_owner_total);
        if owner_residual > 0 {
            escrow::release(env, &event.token, &event.owner, owner_residual);
            evt::OwnerResidualRefunded {
                event_id,
                owner: event.owner.clone(),
                amount: owner_residual,
            }
            .publish(env);
        }
    }

    event.remaining_escrow = 0;
    event.status = EventStatus::Cancelled;
    storage::set_event(env, event_id, &event);
    storage::clear_cancellation_state(env, event_id);

    evt::EventCancelled { id: event_id }.publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// SUBMIT
// ============================================================
pub fn submit(
    env: &Env,
    event_id: u64,
    applicant: Address,
    content_uri: String,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    // Crowdfunding has no submission concept: the builder is pre-seated.
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }
    if let Some(deadline) = event.deadline {
        if deadline <= env.ledger().timestamp() {
            return Err(Error::DeadlinePassed);
        }
    }

    applicant.require_auth();

    let existing = storage::get_submission(env, event_id, &applicant);

    // Pillar-aware application gate (only enforced on first submission).
    // O(1) lookup via the slot index.
    if existing.is_none() {
        let needs_application = matches!(event.pillar, Pillar::Bounty | Pillar::Grant);
        if needs_application && storage::applicant_slot(env, event_id, &applicant) == 0 {
            return Err(Error::ApplicantNotApplied);
        }
    }

    let submitted_at = existing
        .as_ref()
        .map(|s| s.submitted_at)
        .unwrap_or_else(|| env.ledger().timestamp());

    let submission = Submission {
        applicant: applicant.clone(),
        content_uri: content_uri.clone(),
        submitted_at,
    };
    storage::set_submission(env, event_id, &applicant, &submission);

    evt::Submitted {
        event_id,
        applicant,
        content_uri,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// WITHDRAW SUBMISSION
// ============================================================
pub fn withdraw_submission(
    env: &Env,
    event_id: u64,
    applicant: Address,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if let Some(deadline) = event.deadline {
        if deadline <= env.ledger().timestamp() {
            return Err(Error::DeadlinePassed);
        }
    }

    applicant.require_auth();

    if storage::get_submission(env, event_id, &applicant).is_none() {
        return Err(Error::SubmissionNotFound);
    }

    storage::remove_submission(env, event_id, &applicant);

    evt::SubmissionWithdrawn {
        event_id,
        applicant,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// SELECT WINNERS
// ============================================================
pub fn select_winners(
    env: &Env,
    event_id: u64,
    winners: Vec<WinnerSpec>,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    // Management authority: the per-event manager override if set, else owner.
    resolve_manager(env, event_id, &event.owner).require_auth();

    // One-shot per event: detect a prior selection by an anchor row
    // (milestone == None). Crowdfunding's create_event seeds an anchor too,
    // but we returned above for crowdfunding.
    let existing_count = storage::winner_count(env, event_id);
    for idx in 0..existing_count {
        if let Some(w) = storage::winner_at(env, event_id, idx) {
            if w.milestone.is_none() {
                return Err(Error::WinnersAlreadySelected);
            }
        }
    }

    if winners.is_empty() {
        return Err(Error::NoSubmissions);
    }
    if winners.len() > MAX_WINNERS_PER_SELECT {
        return Err(Error::InvalidWinnerPosition);
    }

    // Validate each position exists in distribution and no duplicates.
    let mut seen_positions: Vec<u32> = Vec::new(env);
    for spec in winners.iter() {
        let mut already = false;
        for p in seen_positions.iter() {
            if p == spec.position {
                already = true;
                break;
            }
        }
        if already {
            return Err(Error::DuplicateWinnerPosition);
        }
        if event.winner_distribution.get(spec.position).is_none() {
            return Err(Error::InvalidWinnerPosition);
        }
        seen_positions.push_back(spec.position);
    }

    let profile = profile_client::client(env);
    let now = env.ledger().timestamp();
    let reason_win = Symbol::new(env, "win");

    match event.release_kind {
        ReleaseKind::Single => {
            // M1: percent math is against the live escrow at select time
            // (snapshotted before the first refund), so partner top-ups via
            // add_funds flow into winner payouts rather than getting
            // trapped until cancel. Snapshot once so each winner gets the
            // intended share regardless of the order of releases inside
            // this same call.
            let escrow_at_select = event.remaining_escrow;

            // First pass: compute per-winner amounts and verify total fits.
            let mut total_owed: i128 = 0;
            for spec in winners.iter() {
                let percent = event.winner_distribution.get(spec.position).unwrap() as i128;
                let amount = escrow_at_select.saturating_mul(percent) / 100_i128;
                if amount <= 0 {
                    return Err(Error::InvalidDistribution);
                }
                total_owed = total_owed.saturating_add(amount);
            }
            if total_owed > event.remaining_escrow {
                return Err(Error::InsufficientEscrow);
            }

            // Second pass: release per winner with all four profile-side calls.
            for (idx, spec) in winners.iter().enumerate() {
                let sub_idx = idx as u8;
                let percent = event.winner_distribution.get(spec.position).unwrap() as i128;
                let amount = escrow_at_select.saturating_mul(percent) / 100_i128;

                escrow::release(env, &event.token, &spec.recipient, amount);
                event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);

                let bootstrap_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::BOOTSTRAP, sub_idx);
                profile.bootstrap(&spec.recipient, &bootstrap_op);

                let earn_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::EARN_CREDITS, sub_idx);
                profile.earn_credits(
                    &spec.recipient,
                    &spec.credit_earn,
                    &reason_win,
                    &earn_op,
                );

                let rep_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::BUMP_REP, sub_idx);
                profile.bump_reputation(
                    &spec.recipient,
                    &spec.reputation_bump,
                    &reason_win,
                    &rep_op,
                );

                let earnings_op = idempotency::derive_child_indexed(
                    env,
                    &op_id,
                    tag::REGISTER_EARNINGS,
                    sub_idx,
                );
                profile.register_earnings(
                    &spec.recipient,
                    &event.token,
                    &amount,
                    &earnings_op,
                );

                storage::append_winner(
                    env,
                    event_id,
                    &Winner {
                        recipient: spec.recipient.clone(),
                        position: spec.position,
                        amount,
                        milestone: None,
                        paid_at: Some(now),
                    },
                );

                evt::WinnerPaid {
                    event_id,
                    recipient: spec.recipient.clone(),
                    position: spec.position,
                    amount,
                    milestone: None,
                }
                .publish(env);
            }

            if event.remaining_escrow == 0 {
                event.status = EventStatus::Completed;
            }
        }
        ReleaseKind::Multi(_) => {
            // Grants: record winners but defer payment to claim_milestone.
            for spec in winners.iter() {
                storage::append_winner(
                    env,
                    event_id,
                    &Winner {
                        recipient: spec.recipient.clone(),
                        position: spec.position,
                        amount: 0,
                        milestone: None,
                        paid_at: None,
                    },
                );
            }
        }
    }

    let winners_count = winners.len();
    storage::set_event(env, event_id, &event);

    evt::WinnersSelected {
        event_id,
        count: winners_count,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// READS
//
// Aggregated reads (get_applicants, get_winners, get_contributors) cap at
// MAX_*_PER_EVENT entries. Callers expecting larger lists should use the
// paged accessors (*_count + *_at).
// ============================================================
pub fn get_event(env: &Env, event_id: u64) -> Result<EventRecord, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)
}

pub fn get_submission(env: &Env, event_id: u64, applicant: Address) -> Result<Submission, Error> {
    storage::get_submission(env, event_id, &applicant).ok_or(Error::SubmissionNotFound)
}

pub fn get_applicants(env: &Env, event_id: u64) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::applicants_snapshot(
        env,
        event_id,
        MAX_APPLICANTS_PER_EVENT,
    ))
}

pub fn get_applicant_count(env: &Env, event_id: u64) -> Result<u32, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::applicant_count(env, event_id))
}

pub fn get_applicant_at(env: &Env, event_id: u64, idx: u32) -> Result<Option<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::applicant_at(env, event_id, idx))
}

pub fn get_winners(env: &Env, event_id: u64) -> Result<Vec<Winner>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::winners_snapshot(
        env,
        event_id,
        MAX_WINNERS_PER_SELECT.saturating_mul(20),
    ))
}

pub fn get_winner_count(env: &Env, event_id: u64) -> Result<u32, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::winner_count(env, event_id))
}

pub fn get_winner_at(env: &Env, event_id: u64, idx: u32) -> Result<Option<Winner>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::winner_at(env, event_id, idx))
}

pub fn get_contributors(env: &Env, event_id: u64) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::contributors_snapshot(
        env,
        event_id,
        MAX_CONTRIBUTORS_PER_EVENT,
    ))
}

pub fn get_contributor_count(env: &Env, event_id: u64) -> Result<u32, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::contributor_count(env, event_id))
}

pub fn get_contributor_at(env: &Env, event_id: u64, idx: u32) -> Result<Option<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::contributor_at(env, event_id, idx))
}

pub fn get_contributor_amount(
    env: &Env,
    event_id: u64,
    contributor: Address,
) -> Result<i128, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::get_contributor_amount(env, event_id, &contributor))
}

// Silence unused-import warnings until the stubbed ops land.
#[allow(dead_code)]
const _MARK_USED: (Option<Symbol>,) = (None,);
