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
    PrizeAward, ReleaseKind, Submission, Winner, WinnerSpec,
};

const MAX_TITLE_LEN: u32 = 120;

// Per-call winner cap. Since 1.3.0 (#61) Single-release selection is
// batchable: each position is awardable exactly once, so an event's total
// winner count is bounded by its winner_distribution, and each call stays
// within this known-safe per-transaction bound.
const MAX_WINNERS_PER_SELECT: u32 = 50;

// How long selected winners have to claim before the manager may cancel
// the event and sweep unclaimed prizes back through the refund path.
// Anchored at selection time (not the event deadline, which typically
// passes before winners are even selected). Refreshed by each selection
// batch. Constant for now; a per-event override belongs on EventRecord and
// must wait for the next migration window (see BACKLOG).
pub const PRIZE_CLAIM_WINDOW_SECS: u64 = 90 * 24 * 60 * 60;

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

    // Pull-model gate (1.3.0, #61): while selected prizes remain unclaimed
    // and the claim window is open, the escrow is owed to winners and the
    // manager must not cancel it out from under them. Once the window
    // expires, cancellation proceeds and unclaimed amounts flow back
    // through the normal refund path, so escrow can never be stranded.
    if matches!(event.release_kind, ReleaseKind::Single)
        && storage::unclaimed_prize_count(env, event_id) > 0
    {
        let expiry = storage::get_prize_claim_expiry(env, event_id).unwrap_or(0);
        if env.ledger().timestamp() <= expiry {
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
    match event.release_kind {
        // Single-release selection is batchable on the pull model: the
        // per-position award key is the replay lock. An event with winner
        // rows but no base-escrow key was selected by the pre-1.3.0 push
        // model; keep those one-shot.
        ReleaseKind::Single => {
            if existing_count > 0 && storage::get_prize_base_escrow(env, event_id).is_none() {
                return Err(Error::WinnersAlreadySelected);
            }
        }
        ReleaseKind::Multi(_) => {
            for idx in 0..existing_count {
                if let Some(w) = storage::winner_at(env, event_id, idx) {
                    if w.milestone.is_none() {
                        return Err(Error::WinnersAlreadySelected);
                    }
                }
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

    let now = env.ledger().timestamp();

    match event.release_kind {
        ReleaseKind::Single => {
            // Pull model (1.3.0, #61): record each winner's prize against a
            // baseline fixed at the first selection; the token transfer and
            // profile effects happen in claim_prize, one transaction per
            // winner. The distribution sums to exactly 100 and every
            // position is awardable once, so the sum of recorded amounts
            // can never exceed the baseline.
            let base_escrow = match storage::get_prize_base_escrow(env, event_id) {
                Some(b) => b,
                None => {
                    let b = event.remaining_escrow;
                    storage::set_prize_base_escrow(env, event_id, b);
                    b
                }
            };

            let mut total_owed: i128 = 0;
            for spec in winners.iter() {
                if storage::get_prize_award(env, event_id, spec.position).is_some() {
                    return Err(Error::DuplicateWinnerPosition);
                }
                let percent = event
                    .winner_distribution
                    .get(spec.position)
                    .ok_or(Error::InvalidDistribution)? as i128;
                let amount = base_escrow.saturating_mul(percent) / 100_i128;
                if amount <= 0 {
                    return Err(Error::InvalidDistribution);
                }
                total_owed = total_owed.saturating_add(amount);
            }
            if total_owed > event.remaining_escrow {
                return Err(Error::InsufficientEscrow);
            }

            for (idx, spec) in winners.iter().enumerate() {
                let percent = event
                    .winner_distribution
                    .get(spec.position)
                    .ok_or(Error::InvalidDistribution)? as i128;
                let amount = base_escrow.saturating_mul(percent) / 100_i128;

                let anchor_idx = existing_count + (idx as u32);
                storage::append_winner(
                    env,
                    event_id,
                    &Winner {
                        recipient: spec.recipient.clone(),
                        position: spec.position,
                        amount,
                        milestone: None,
                        paid_at: None,
                    },
                );
                storage::set_prize_award(
                    env,
                    event_id,
                    spec.position,
                    &PrizeAward {
                        recipient: spec.recipient.clone(),
                        anchor_idx,
                        reputation_bump: spec.reputation_bump,
                    },
                );
            }

            let unclaimed = storage::unclaimed_prize_count(env, event_id);
            storage::set_unclaimed_prize_count(
                env,
                event_id,
                unclaimed.saturating_add(winners.len()),
            );

            // Refresh the claim window so late-selected batches get the
            // full window; start_cancel stays blocked until it expires.
            let expiry = now.saturating_add(PRIZE_CLAIM_WINDOW_SECS);
            let cur = storage::get_prize_claim_expiry(env, event_id).unwrap_or(0);
            if expiry > cur {
                storage::set_prize_claim_expiry(env, event_id, expiry);
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
// CLAIM PRIZE (pull model for Single-release events; 1.3.0, #61)
//
// One winner per transaction: the recorded recipient authorizes, the
// amount fixed at selection time is released from escrow, and profile
// effects run best-effort after payment so a profile-contract failure can
// never strand the payout. Mirrors the claim_milestone pull pattern used
// by Multi-release events. WinnerPaid keeps firing at the moment money
// moves, so off-chain consumers are unchanged.
// ============================================================
pub fn claim_prize(
    env: &Env,
    event_id: u64,
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

    let award =
        storage::get_prize_award(env, event_id, position).ok_or(Error::InvalidWinnerPosition)?;
    award.recipient.require_auth();

    let anchor =
        storage::winner_at(env, event_id, award.anchor_idx).ok_or(Error::InvalidWinnerPosition)?;
    if anchor.recipient != award.recipient || anchor.position != position {
        return Err(Error::InvalidWinnerPosition);
    }
    if anchor.paid_at.is_some() {
        return Err(Error::PrizeAlreadyClaimed);
    }
    let amount = anchor.amount;
    if amount <= 0 {
        return Err(Error::InvalidDistribution);
    }
    if amount > event.remaining_escrow {
        return Err(Error::InsufficientEscrow);
    }

    let now = env.ledger().timestamp();
    storage::set_winner_at(
        env,
        event_id,
        award.anchor_idx,
        &Winner {
            recipient: anchor.recipient.clone(),
            position,
            amount,
            milestone: None,
            paid_at: Some(now),
        },
    );

    let unclaimed = storage::unclaimed_prize_count(env, event_id);
    storage::set_unclaimed_prize_count(env, event_id, unclaimed.saturating_sub(1));

    event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);
    if event.remaining_escrow == 0 {
        event.status = EventStatus::Completed;
    }
    storage::set_event(env, event_id, &event);
    idempotency::mark_seen(env, &op_id);

    // Interactions after effects. A failed token transfer traps and rolls
    // the whole invocation back, so ordering costs nothing.
    escrow::release(env, &event.token, &award.recipient, amount);

    evt::WinnerPaid {
        event_id,
        recipient: award.recipient.clone(),
        position,
        amount,
        milestone: None,
    }
    .publish(env);

    // Best-effort profile effects: funds already moved, so a profile
    // failure must not fail the claim.
    let profile = profile_client::client(env);
    let reason_win = Symbol::new(env, "win");

    let bootstrap_op = idempotency::derive_child(env, &op_id, tag::BOOTSTRAP);
    let _ = profile.try_bootstrap(&award.recipient, &bootstrap_op);

    let rep_op = idempotency::derive_child(env, &op_id, tag::BUMP_REP);
    let _ = profile.try_bump_reputation(
        &award.recipient,
        &award.reputation_bump,
        &reason_win,
        &rep_op,
    );

    let earnings_op = idempotency::derive_child(env, &op_id, tag::REGISTER_EARNINGS);
    let _ = profile.try_register_earnings(&award.recipient, &event.token, &amount, &earnings_op);

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

#[allow(dead_code)]
const _MARK_USED: (Option<Symbol>,) = (None,);
