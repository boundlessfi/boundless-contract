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

// Max winners per select_winners call. With the pull-model (2026-07),
// select_winners only records winners (1 storage write each) and defers
// all token transfers + cross-contract calls to per-winner claim_prize
// transactions. However, claim_prize still performs a linear scan of the
// winner list to locate the anchor row, and get_winners loads all rows
// into a Vec. Without keyed winner lookup or batched recording, 500 is a
// safe ceiling compatible with the current table-backed implementation.
pub const MAX_WINNERS_PER_SELECT: u32 = 500;

pub const MAX_APPLICANTS_PER_EVENT: u32 = 5_000;
pub const MAX_CONTRIBUTORS_PER_EVENT: u32 = 5_000;

pub const MAX_REFUNDS_PER_BATCH: u32 = 25;

const MIN_CONTRIBUTION_STROOPS: i128 = 100_000_000_i128;

// ============================================================
// CREATE EVENT
// ============================================================
fn resolve_manager(env: &Env, event_id: u64, owner: &Address) -> Address {
    storage::get_event_manager(env, event_id).unwrap_or_else(|| owner.clone())
}

fn get_or_init_non_owner_total(env: &Env, event_id: u64) -> Result<i128, Error> {
    match storage::get_non_owner_contribution_total(env, event_id) {
        Some(total) => Ok(total),
        None if storage::contributor_count(env, event_id) == 0 => {
            storage::set_non_owner_contribution_total(env, event_id, 0);
            Ok(0)
        }
        None => Err(Error::CancellationTotalMissing),
    }
}

pub fn create_event(env: &Env, params: CreateEventParams, op_id: BytesN<32>) -> Result<u64, Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    params.owner.require_auth();

    token_whitelist::require_supported(env, &params.token)?;

    if params.title.len() > MAX_TITLE_LEN {
        return Err(Error::TitleTooLong);
    }

    if params.total_budget <= 0 {
        return Err(Error::InvalidBudget);
    }

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

    if let Some(deadline) = params.deadline {
        if deadline <= env.ledger().timestamp() {
            return Err(Error::DeadlineMustBeFuture);
        }
    }

    if let Some(bps) = params.fee_bps_override {
        if bps > MAX_FEE_BPS {
            return Err(Error::InvalidFeeBps);
        }
    }
    let effective_bps = escrow::effective_fee_bps(env, params.fee_bps_override);

    let is_crowdfunding = matches!(params.pillar, Pillar::Crowdfunding);
    let initial_escrow: i128 = if is_crowdfunding {
        0
    } else {
        params.total_budget
    };

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

    let id = idempotency::next_event_id(env);
    let record = EventRecord { id, ..provisional };
    storage::set_event(env, id, &record);
    storage::set_non_owner_contribution_total(env, id, 0);

    if let Some(manager) = &params.manager {
        storage::set_event_manager(env, id, manager);
    }

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
                reputation_bump: None,
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

pub fn set_manager(env: &Env, event_id: u64, new_manager: Address) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    resolve_manager(env, event_id, &event.owner).require_auth();
    storage::set_event_manager(env, event_id, &new_manager);
    Ok(())
}

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

    let is_non_owner = from != event.owner;
    let prior_contribution = if is_non_owner {
        storage::get_contributor_amount(env, event_id, &from)
    } else {
        0
    };
    let non_owner_total_before = if is_non_owner {
        get_or_init_non_owner_total(env, event_id)?
    } else {
        0
    };

    if is_non_owner {
        let prior = prior_contribution;
        if prior == 0 {
            storage::append_contributor(env, event_id, &from, MAX_CONTRIBUTORS_PER_EVENT)?;
        }
    }

    let credited = if matches!(event.pillar, Pillar::Crowdfunding) {
        escrow::deposit_no_fee(env, &event.token, &from, amount)
    } else {
        let effective_bps = escrow::effective_fee_bps(env, event.fee_bps_override);
        escrow::deposit_with_fee_at(env, &event.token, &from, amount, effective_bps)
    };
    event.remaining_escrow = event.remaining_escrow.saturating_add(credited);

    if is_non_owner {
        let new_total = prior_contribution.saturating_add(credited);
        storage::set_contributor_amount(env, event_id, &from, new_total);
        storage::set_non_owner_contribution_total(
            env,
            event_id,
            non_owner_total_before.saturating_add(credited),
        );
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

/// True if the Single-release event has at least one winner anchor whose
/// prize has not yet been claimed (`paid_at` is `None`).  The manager must
/// not be able to cancel and drain the escrow before those winners claim.
fn has_unclaimed_single_winner(env: &Env, event_id: u64) -> bool {
    let count = storage::winner_count(env, event_id);
    for idx in 0..count {
        if let Some(w) = storage::winner_at(env, event_id, idx) {
            if w.milestone.is_none() && w.paid_at.is_none() {
                return true;
            }
        }
    }
    false
}

// ============================================================
// PAGED CANCEL
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

    // Prevent cancel after winners selected for Single-release (pull-model).
    // The manager must not be able to drain escrow before winners claim,
    // unless the event deadline has passed (liveness escape hatch for
    // winners who cannot authenticate).
    if matches!(event.release_kind, ReleaseKind::Single) && has_unclaimed_single_winner(env, event_id)
    {
        let deadline_passed = event
            .deadline
            .map_or(false, |d| env.ledger().timestamp() > d);
        if !deadline_passed {
            return Err(Error::WinnersAlreadySelected);
        }
    }

    resolve_manager(env, event_id, &event.owner).require_auth();

    let remaining = event.remaining_escrow;
    let count = storage::contributor_count(env, event_id);
    let non_owner_total = get_or_init_non_owner_total(env, event_id)?;

    let branch = if non_owner_total <= 0 {
        CancellationBranch::OwnerOnly
    } else if remaining >= non_owner_total {
        CancellationBranch::FullPartnerThenResidual
    } else {
        CancellationBranch::ProRataPartners
    };

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
        storage::set_non_owner_contribution_total(env, event_id, 0);
        evt::EventCancelled { id: event_id }.publish(env);
        idempotency::mark_seen(env, &op_id);
        return Ok(());
    }

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

    let cap = if max_refunds > MAX_REFUNDS_PER_BATCH {
        MAX_REFUNDS_PER_BATCH
    } else {
        max_refunds
    };
    let mut processed: u32 = 0;

    let mut state =
        storage::get_cancellation_state(env, event_id).ok_or(Error::CancellationNotStarted)?;

    while processed < cap && state.next_idx < state.count_at_start {
        let idx = state.next_idx;
        state.next_idx = state.next_idx.saturating_add(1);
        processed = processed.saturating_add(1);

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
    let state =
        storage::get_cancellation_state(env, event_id).ok_or(Error::CancellationNotStarted)?;
    if state.next_idx < state.count_at_start {
        return Err(Error::CancellationNotFinished);
    }

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
    storage::set_non_owner_contribution_total(env, event_id, 0);

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

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    resolve_manager(env, event_id, &event.owner).require_auth();

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

    match event.release_kind {
        ReleaseKind::Single => {
            // Pull-model (2026-07): record winners with pre-computed amounts
            // but defer all token releases + profile calls to claim_prize.
            let escrow_at_select = event.remaining_escrow;

            let mut total_owed: i128 = 0;
            for spec in winners.iter() {
                let percent = event
                    .winner_distribution
                    .get(spec.position)
                    .ok_or(Error::InvalidDistribution)? as i128;
                let amount = escrow_at_select.saturating_mul(percent) / 100_i128;
                if amount <= 0 {
                    return Err(Error::InvalidDistribution);
                }
                total_owed = total_owed.saturating_add(amount);
            }
            if total_owed > event.remaining_escrow {
                return Err(Error::InsufficientEscrow);
            }

            for (i, spec) in winners.iter().enumerate() {
                let percent = event
                    .winner_distribution
                    .get(spec.position)
                    .ok_or(Error::InvalidDistribution)? as i128;
                let amount = escrow_at_select.saturating_mul(percent) / 100_i128;

                let anchor_idx = existing_count + (i as u32);
                storage::append_winner(
                    env,
                    event_id,
                    &Winner {
                        recipient: spec.recipient.clone(),
                        position: spec.position,
                        amount,
                        milestone: None,
                        paid_at: None,
                        reputation_bump: Some(spec.reputation_bump),
                    },
                );
                storage::set_winner_index(
                    env,
                    event_id,
                    &spec.recipient,
                    spec.position,
                    anchor_idx,
                );
            }
        }
        ReleaseKind::Multi(_) => {
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
                        reputation_bump: Some(spec.reputation_bump),
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
// CLAIM PRIZE (pull-model for Single-release events)
//
// Each winner claims their individual prize in their own transaction,
// so there is no per-call winner ceiling. select_winners records winners
// with pre-computed amounts; this function releases the token, bumps
// reputation, and registers earnings for one winner per call.
//
// This mirrors claim_milestone but for ReleaseKind::Single (bounties,
// hackathons) instead of ReleaseKind::Multi (grants, crowdfunding).
// ============================================================
pub fn claim_prize(
    env: &Env,
    event_id: u64,
    recipient: Address,
    position: u32,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if !matches!(event.release_kind, ReleaseKind::Single) {
        return Err(Error::InvalidReleaseKind);
    }
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    recipient.require_auth();

    // Prevent double-claim for this (event, recipient, position).
    if storage::is_prize_claimed(env, event_id, &recipient, position) {
        return Err(Error::PrizeAlreadyClaimed);
    }

    // Look up the anchor index stored at selection time (O(1) instead of
    // a linear scan). Returns NoSubmissions if no winner matches or the
    // prize has already been claimed (canonical guard: paid_at).
    let anchor_idx = storage::get_winner_index(env, event_id, &recipient, position)
        .ok_or(Error::NoSubmissions)?;
    let w = storage::winner_at(env, event_id, anchor_idx)
        .ok_or(Error::NoSubmissions)?;
    if w.recipient != recipient || w.position != position || w.milestone.is_some() {
        return Err(Error::NoSubmissions);
    }
    if w.paid_at.is_some() {
        return Err(Error::PrizeAlreadyClaimed);
    }
    let amount = w.amount;
    let reputation_bump = w.reputation_bump.unwrap_or(0);

    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }
    if amount > event.remaining_escrow {
        return Err(Error::InsufficientEscrow);
    }

    // Release token from escrow.
    escrow::release(env, &event.token, &recipient, amount);
    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);

    // Cross-contract profile mutations. Each call gets a unique child op_id.
    let profile = profile_client::client(env);
    let reason_win = Symbol::new(env, "win");

    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    profile.bootstrap(&recipient, &bootstrap_op);

    let rep_op = idempotency::derive_child(env, &op_id, tag::BUMP_REP);
    profile.bump_reputation(&recipient, &reputation_bump, &reason_win, &rep_op);

    let earnings_op = idempotency::derive_child(env, &op_id, tag::REGISTER_EARNINGS);
    profile.register_earnings(&recipient, &event.token, &amount, &earnings_op);

    // Mark prize claimed (prevents replay for this recipient + position).
    storage::mark_prize_claimed(env, event_id, &recipient, position);

    // Update the anchor row in-place with the paid timestamp instead of
    // appending a duplicate row. This keeps winner_count == selected count.
    storage::set_winner_at(
        env,
        event_id,
        anchor_idx,
        &Winner {
            recipient: recipient.clone(),
            position,
            amount,
            milestone: None,
            paid_at: Some(env.ledger().timestamp()),
            reputation_bump: Some(reputation_bump),
        },
    );

    if event.remaining_escrow == 0 {
        event.status = EventStatus::Completed;
    }
    storage::set_event(env, event_id, &event);

    evt::PrizeClaimed {
        event_id,
        recipient,
        position,
        amount,
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// READS
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
        MAX_WINNERS_PER_SELECT,
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

#[allow(dead_code)]
const _MARK_USED: (Option<Symbol>,) = (None,);
