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
    CancellationBranch, CancellationState, CreateEventParams, EventRecord, EventStatus,
    PendingManager, Pillar, PrizeAward, ReleaseKind, Submission, Winner, WinnerSpec,
};

const MAX_TITLE_LEN: u32 = 120;

const MAX_WINNERS_PER_SELECT: u32 = 50;

const PENDING_MANAGER_TTL_LEDGERS: u32 = 17_280;

// Anchored at selection time, not the event deadline (which usually passes
// before winners are selected). A per-event override needs a migration.
pub const PRIZE_CLAIM_WINDOW_SECS: u64 = 90 * 24 * 60 * 60;

// Participant sets (applicants, contributors, submissions) are unbounded:
// each entry is its own persistent ledger entry paid for by the participant's
// own transaction, and no state-changing path iterates the full set in one
// transaction (refunds are cranked in batches, winner selection takes an
// explicit bounded list). Full-list reads page through VIEW_PAGE_LIMIT
// entries per call so simulation stays inside per-tx read-entry limits.
pub const VIEW_PAGE_LIMIT: u32 = 100;
pub const MAX_CONTENT_URI_LEN: u32 = 256;

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

    params.owner.require_auth();
    idempotency::require_unseen(env, &params.owner, &op_id)?;

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

    let id = idempotency::next_event_id(env)?;
    let record = EventRecord { id, ..provisional };
    storage::set_event(env, id, &record);
    storage::set_non_owner_contribution_total(env, id, 0);

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

    if let Some(manager) = &params.manager {
        let expires_at = env
            .ledger()
            .sequence()
            .saturating_add(PENDING_MANAGER_TTL_LEDGERS);
        let pending = PendingManager {
            target: manager.clone(),
            expires_at_ledger: expires_at,
        };
        storage::set_pending_manager(env, id, &pending);
        evt::ManagerProposed {
            event_id: id,
            target: manager.clone(),
            expires_at_ledger: expires_at,
        }
        .publish(env);
    }

    idempotency::mark_seen(env, &params.owner, &op_id);
    Ok(id)
}

// ============================================================
// MANAGER ROTATION (two-step propose / accept)
// ============================================================
pub fn propose_manager(env: &Env, event_id: u64, new_manager: Address) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    resolve_manager(env, event_id, &event.owner).require_auth();

    let expires_at = env
        .ledger()
        .sequence()
        .saturating_add(PENDING_MANAGER_TTL_LEDGERS);
    let pending = PendingManager {
        target: new_manager.clone(),
        expires_at_ledger: expires_at,
    };
    storage::set_pending_manager(env, event_id, &pending);

    evt::ManagerProposed {
        event_id,
        target: new_manager,
        expires_at_ledger: expires_at,
    }
    .publish(env);
    Ok(())
}

pub fn accept_manager(env: &Env, event_id: u64) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;

    let pending =
        storage::get_pending_manager(env, event_id).ok_or(Error::PendingRotationMismatch)?;

    if env.ledger().sequence() > pending.expires_at_ledger {
        storage::clear_pending_manager(env, event_id);
        return Err(Error::PendingRotationExpired);
    }

    pending.target.require_auth();

    storage::set_event_manager(env, event_id, &pending.target);
    storage::clear_pending_manager(env, event_id);

    evt::ManagerChanged {
        event_id,
        new_manager: pending.target,
    }
    .publish(env);
    Ok(())
}

pub fn cancel_pending_manager(env: &Env, event_id: u64) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    resolve_manager(env, event_id, &event.owner).require_auth();

    if storage::get_pending_manager(env, event_id).is_none() {
        return Err(Error::PendingRotationMismatch);
    }
    storage::clear_pending_manager(env, event_id);

    evt::PendingManagerCancelled { event_id }.publish(env);
    Ok(())
}

pub fn get_manager(env: &Env, event_id: u64) -> Result<Address, Error> {
    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(resolve_manager(env, event_id, &event.owner))
}

pub fn get_pending_manager(env: &Env, event_id: u64) -> Result<Option<PendingManager>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::get_pending_manager(env, event_id))
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
    idempotency::require_unseen(env, &from, &op_id)?;

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
            storage::append_contributor(env, event_id, &from)?;
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
        contributor: from.clone(),
        amount: credited,
        new_remaining: event.remaining_escrow,
    }
    .publish(env);

    idempotency::mark_seen(env, &from, &op_id);
    Ok(())
}

// ============================================================
// PAGED CANCEL
// ============================================================
pub fn start_cancel(env: &Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_not_paused(env)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if storage::get_cancellation_state(env, event_id).is_some() {
        return Err(Error::CancellationAlreadyStarted);
    }

    // Block cancel while prizes are unclaimed and the window is open;
    // after it expires, unclaimed amounts sweep out via the refund path.
    if matches!(event.release_kind, ReleaseKind::Single)
        && storage::unclaimed_prize_count(env, event_id) > 0
    {
        let expiry = storage::get_prize_claim_expiry(env, event_id).unwrap_or(0);
        if env.ledger().timestamp() <= expiry {
            return Err(Error::WinnersAlreadySelected);
        }
    }

    let manager = resolve_manager(env, event_id, &event.owner);
    manager.require_auth();
    idempotency::require_unseen(env, &manager, &op_id)?;

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
        idempotency::mark_seen(env, &manager, &op_id);
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

    idempotency::mark_seen(env, &manager, &op_id);
    Ok(())
}

pub fn process_cancel_batch(
    env: &Env,
    event_id: u64,
    max_refunds: u32,
    op_id: BytesN<32>,
) -> Result<u32, Error> {
    admin::require_not_paused(env)?;
    // Permissionless crank: no caller to authorize, so namespace under the
    // contract's own address — isolated from every user/privileged domain.
    let domain = env.current_contract_address();
    idempotency::require_unseen(env, &domain, &op_id)?;

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

    idempotency::mark_seen(env, &domain, &op_id);
    Ok(remaining_to_process)
}

pub fn finalize_cancel(env: &Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    let domain = env.current_contract_address();
    idempotency::require_unseen(env, &domain, &op_id)?;

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

    idempotency::mark_seen(env, &domain, &op_id);
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

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    applicant.require_auth();
    idempotency::require_unseen(env, &applicant, &op_id)?;

    // Reused rather than adding a new variant — stays inside the
    // contracterror 50-variant cap (see BACKLOG.md L7 for precedent).
    if content_uri.len() > MAX_CONTENT_URI_LEN {
        return Err(Error::TitleTooLong);
    }

    let existing = storage::get_submission(env, event_id, &applicant);

    if existing.is_none() {
        let needs_application = matches!(event.pillar, Pillar::Bounty | Pillar::Grant);
        if needs_application && storage::applicant_slot(env, event_id, &applicant) == 0 {
            return Err(Error::ApplicantNotApplied);
        }
    }

    // Count the submission before writing. There is no cap: each submission
    // is its own ledger entry whose write and rent are paid by the
    // submitter's transaction, so spam addresses fund their own storage and
    // cannot lock real participants out of a full event.
    storage::append_submission(env, event_id, &applicant)?;

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
        applicant: applicant.clone(),
        content_uri,
    }
    .publish(env);

    idempotency::mark_seen(env, &applicant, &op_id);
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

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }

    applicant.require_auth();
    idempotency::require_unseen(env, &applicant, &op_id)?;

    if storage::get_submission(env, event_id, &applicant).is_none() {
        return Err(Error::SubmissionNotFound);
    }

    storage::remove_submission(env, event_id, &applicant);

    evt::SubmissionWithdrawn {
        event_id,
        applicant: applicant.clone(),
    }
    .publish(env);

    idempotency::mark_seen(env, &applicant, &op_id);
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

    let event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    let manager = resolve_manager(env, event_id, &event.owner);
    manager.require_auth();
    idempotency::require_unseen(env, &manager, &op_id)?;

    let existing_count = storage::winner_count(env, event_id);
    match event.release_kind {
        ReleaseKind::Single => {
            // Winner rows but no base-escrow key means a pre-1.3.0 push-model
            // event: keep it one-shot. New events award each position once.
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
            // Amounts are fixed against the escrow baseline captured at the
            // first selection; claim_prize does the transfer and profile calls.
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

                let anchor_idx = existing_count
                    .checked_add(idx as u32)
                    .ok_or(Error::TooManyContributors)?;
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

            // Extend the window so a later batch's winners get the full term.
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

    idempotency::mark_seen(env, &manager, &op_id);
    Ok(())
}

// ============================================================
// CLAIM PRIZE (pull model for Single-release events; #61)
// ============================================================
pub fn claim_prize(
    env: &Env,
    event_id: u64,
    position: u32,
    op_id: BytesN<32>,
) -> Result<(), Error> {
    admin::require_not_paused(env)?;

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
    idempotency::require_unseen(env, &award.recipient, &op_id)?;

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
    idempotency::mark_seen(env, &award.recipient, &op_id);

    // State written above; release last so a reentrant token can't double-claim.
    escrow::release(env, &event.token, &award.recipient, amount);

    evt::WinnerPaid {
        event_id,
        recipient: award.recipient.clone(),
        position,
        amount,
        milestone: None,
    }
    .publish(env);

    // Best-effort: the payout is final, so a profile failure must not revert it.
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

// Full-list getters return the first VIEW_PAGE_LIMIT entries; use the
// _page variants (or the per-index getters / the off-chain indexer) to
// read beyond that.
pub fn get_applicants(env: &Env, event_id: u64) -> Result<Vec<Address>, Error> {
    get_applicants_page(env, event_id, 0, VIEW_PAGE_LIMIT)
}

pub fn get_applicants_page(
    env: &Env,
    event_id: u64,
    start: u32,
    limit: u32,
) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::applicants_snapshot(
        env,
        event_id,
        start,
        limit.min(VIEW_PAGE_LIMIT),
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
    get_winners_page(env, event_id, 0, VIEW_PAGE_LIMIT)
}

pub fn get_winners_page(
    env: &Env,
    event_id: u64,
    start: u32,
    limit: u32,
) -> Result<Vec<Winner>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::winners_snapshot(
        env,
        event_id,
        start,
        limit.min(VIEW_PAGE_LIMIT),
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
    get_contributors_page(env, event_id, 0, VIEW_PAGE_LIMIT)
}

pub fn get_contributors_page(
    env: &Env,
    event_id: u64,
    start: u32,
    limit: u32,
) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::contributors_snapshot(
        env,
        event_id,
        start,
        limit.min(VIEW_PAGE_LIMIT),
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
