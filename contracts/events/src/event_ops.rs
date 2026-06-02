// boundless-events: canonical event lifecycle operations.
//
// Spec: boundless-platform-contract-prd.md Sections 6.2, 6.4, 6.5, 7.

use soroban_sdk::{Address, BytesN, Env, String, Symbol, Vec};

use crate::admin;
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
    CreateEventParams, EventRecord, EventStatus, Pillar, ReleaseKind, Submission, Winner,
    WinnerSpec,
};

const MAX_TITLE_LEN: u32 = 120;
const MAX_APPLY_COST: u32 = 100;
const MAX_WINNERS_PER_SELECT: u32 = 50;

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

    // Crowdfunding starts at zero escrow; the field's role flips: total_budget
    // becomes the funding goal, and remaining_escrow grows only through
    // community add_funds calls. Every other pillar deposits the owner's
    // total_budget into escrow at create time.
    let is_crowdfunding = matches!(params.pillar, Pillar::Crowdfunding);
    let initial_escrow: i128 = if is_crowdfunding { 0 } else { params.total_budget };

    // Pillar-specific validation. EventRecord is partially constructed here
    // so the per-pillar validator can inspect release_kind + deadline.
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
    };
    match params.pillar {
        Pillar::Hackathon => hackathon::validate_create(env, &provisional, &params.owner)?,
        Pillar::Bounty => bounty::validate_create(env, &provisional, &params.owner)?,
        Pillar::Grant => grant::validate_create(env, &provisional, &params.owner)?,
        Pillar::Crowdfunding => crowdfunding::validate_create(env, &provisional, &params.owner)?,
    }

    // Deposit funds: pull total_budget + fee from owner, forward fee atomically.
    // Crowdfunding skips this: the campaign starts with zero escrow and is
    // funded entirely through community add_funds.
    if !is_crowdfunding {
        escrow::deposit_with_fee(env, &params.token, &params.owner, params.total_budget);
    }

    // Assign id and persist.
    let id = idempotency::next_event_id(env);
    let record = EventRecord {
        id,
        ..provisional
    };
    storage::set_event(env, id, &record);

    // Crowdfunding: pre-seat the builder as the sole winner at position 1.
    // This skips select_winners entirely: the recipient is fixed at create
    // time so claim_milestone can resolve the payout target without any
    // intermediate organizer-curation step. milestone=None matches the
    // anchor-record convention used by select_winners for Multi(n) releases.
    if is_crowdfunding {
        let mut winners: soroban_sdk::Vec<Winner> = soroban_sdk::Vec::new(env);
        winners.push_back(Winner {
            recipient: params.owner.clone(),
            position: 1,
            amount: 0,
            milestone: None,
            paid_at: None,
        });
        storage::set_winners(env, id, &winners);
    }

    evt::EventCreated {
        id,
        pillar: record.pillar.clone(),
        owner: record.owner.clone(),
        token: record.token.clone(),
        total_budget: record.total_budget,
        content_uri: record.content_uri.clone(),
    }
    .publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(id)
}

// ============================================================
// ADD FUNDS (partner / community contribution)
// ============================================================
//
// Open top-up to an active event's escrow. Anyone with the event id and a
// funded wallet can call this; no pre-registration. The minimum contribution
// is hard-coded at MIN_CONTRIBUTION_STROOPS so that the contributor list and
// pro-rata refund math don't get spammed by dust. Owner top-ups are allowed
// and use the same path: if the caller is event.owner the deposit is folded
// into the running owner balance rather than recorded as a Contribution
// entry (cancel_event refund policy treats the owner as residual; we keep
// the Contribution list clean of owner rows).
//
// Token: contributions must be in event.token. We don't accept cross-token
// top-ups; convert off-chain first.
//
// Fee: the deposit fee bps is charged the same way as create_event, so the
// fee account funds out of every top-up. This keeps the fee model uniform
// across initial deposits and partner contributions.
//
// Refund policy is documented in cancel_event below.
//
// Spec: boundless-partner-contributions-prd.md Sections 5 + 6.
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

    // Pull amount + fee from the contributor and forward the fee to the fee
    // account. escrow::deposit_with_fee returns the net amount credited.
    let credited = escrow::deposit_with_fee(env, &event.token, &from, amount);
    event.remaining_escrow = event.remaining_escrow.saturating_add(credited);

    // Only non-owner contributions land in the ContributorList. Owner top-ups
    // grow remaining_escrow but stay invisible to the partner-refund pass.
    if from != event.owner {
        let prior = storage::get_contributor_amount(env, event_id, &from);
        let new_total = prior.saturating_add(credited);
        storage::set_contributor_amount(env, event_id, &from, new_total);
        if prior == 0 {
            let mut list = storage::get_contributor_list(env, event_id);
            list.push_back(from.clone());
            storage::set_contributor_list(env, event_id, &list);
        }
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
// CANCEL EVENT
// ============================================================
//
// Refund policy (partners-in-full-then-owner-residual):
//
//   non_owner_total = sum(ContributorAmount(event_id, *))
//   remaining       = event.remaining_escrow
//
//   case A — remaining >= non_owner_total:
//     refund each contributor their full amount, then refund (remaining -
//     non_owner_total) to the owner.
//
//   case B — remaining < non_owner_total:
//     pro-rate by amount: contributor c gets floor(c.amount * remaining /
//     non_owner_total). Owner gets nothing. Rounding dust stays in the
//     contract; it's negligible at the 7-decimal scale and the dust accrues
//     to no one until the next admin sweep.
//
// For grants that have already distributed some milestones, only the unspent
// balance is refunded; past milestone payments are not clawed back. The
// policy applies symmetrically: if partners contributed and milestones drained
// the escrow below non_owner_total, partners get pro-rated.
//
// Spec: boundless-platform-contract-prd.md Section 6.2;
//       boundless-partner-contributions-prd.md Section 7.
pub fn cancel_event(env: &Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
    admin::require_not_paused(env)?;
    idempotency::require_unseen(env, &op_id)?;

    let mut event = storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    if !matches!(event.status, EventStatus::Active) {
        return Err(Error::EventNotActive);
    }

    event.owner.require_auth();

    let remaining = event.remaining_escrow;
    if remaining > 0 {
        let contributors = storage::get_contributor_list(env, event_id);

        // Sum non-owner contributions still on the books. We re-read each
        // amount rather than caching: cancel_event is one-shot per event so
        // the extra reads are cheap, and the on-chain value is the source of
        // truth in case ContributorAmount was mutated elsewhere.
        let mut non_owner_total: i128 = 0;
        for c in contributors.iter() {
            non_owner_total = non_owner_total.saturating_add(
                storage::get_contributor_amount(env, event_id, &c),
            );
        }

        if non_owner_total <= 0 {
            // No partner contributions: owner gets everything.
            escrow::release(env, &event.token, &event.owner, remaining);
            evt::OwnerResidualRefunded {
                event_id,
                owner: event.owner.clone(),
                amount: remaining,
            }
            .publish(env);
        } else if remaining >= non_owner_total {
            // Case A: full partner refund + owner residual.
            for c in contributors.iter() {
                let amt = storage::get_contributor_amount(env, event_id, &c);
                if amt > 0 {
                    escrow::release(env, &event.token, &c, amt);
                    storage::set_contributor_amount(env, event_id, &c, 0);
                    evt::ContributorRefunded {
                        event_id,
                        contributor: c.clone(),
                        amount: amt,
                    }
                    .publish(env);
                }
            }
            let owner_residual = remaining.saturating_sub(non_owner_total);
            if owner_residual > 0 {
                escrow::release(env, &event.token, &event.owner, owner_residual);
                evt::OwnerResidualRefunded {
                    event_id,
                    owner: event.owner.clone(),
                    amount: owner_residual,
                }
                .publish(env);
            }
        } else {
            // Case B: pro-rate partners; owner gets nothing.
            for c in contributors.iter() {
                let amt = storage::get_contributor_amount(env, event_id, &c);
                if amt > 0 {
                    let payout = amt.saturating_mul(remaining) / non_owner_total;
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
            }
            // Dust (remaining - sum of payouts) stays in the contract balance.
        }

        event.remaining_escrow = 0;
    }

    event.status = EventStatus::Cancelled;
    storage::set_event(env, event_id, &event);

    evt::EventCancelled { id: event_id }.publish(env);

    idempotency::mark_seen(env, &op_id);
    Ok(())
}

// ============================================================
// SUBMIT
// ============================================================
//
// Upsert semantics: the first submit creates the anchor, later calls update
// content_uri but preserve the original submitted_at. This matches the
// "edit window" model in the Operate PRDs: submissions are editable up to
// the deadline; the audit time is the first submission's timestamp.
//
// Pillar-specific prerequisite:
//   - Bounty / Grant: applicant must have applied (be in EventApplicants)
//                     OR already have a prior submission (re-submit case).
//   - Hackathon:      no prior application required; submission is the entry.
//
// Spec: boundless-platform-contract-prd.md Section 6.3; boundless-hackathon-operate-prd.md FR-4/FR-5; boundless-bounty-operate-prd.md FR-5..FR-7.
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
    // Crowdfunding has no submission concept: the builder is pre-seated as
    // the sole winner at create time and milestone validation is off-chain.
    // Rejecting at the contract layer keeps junk anchors out of storage.
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
    if existing.is_none() {
        let needs_application = matches!(event.pillar, Pillar::Bounty | Pillar::Grant);
        if needs_application {
            let applicants = storage::get_applicants(env, event_id);
            let mut found = false;
            for a in applicants.iter() {
                if a == applicant {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Error::ApplicantNotApplied);
            }
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
//
// Removes the submission anchor. Allowed by the applicant until the event's
// deadline passes (matches the Operate PRDs' withdraw-until-deadline rule).
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
//
// Spec: boundless-platform-contract-prd.md Section 6.4.
//
// Single release: per-winner amount = total_budget * distribution[position] / 100.
//   Tokens transfer immediately; profile is bootstrapped, credits earned,
//   reputation bumped, earnings registered. Event marks Completed when
//   remaining_escrow reaches 0.
//
// Multi release (grant): winners are recorded but no transfer happens here;
//   payments flow through claim_milestone.
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
    // Crowdfunding seats the builder as winner at create time; there is no
    // organizer-curation step. Reject here so the caller cannot overwrite
    // the auto-registered Winner record.
    if matches!(event.pillar, Pillar::Crowdfunding) {
        return Err(Error::InvalidPillar);
    }

    event.owner.require_auth();

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
    let mut winner_records: Vec<Winner> = Vec::new(env);
    let now = env.ledger().timestamp();
    let reason_win = Symbol::new(env, "win");

    match event.release_kind {
        ReleaseKind::Single => {
            // First pass: compute per-winner amounts and verify total fits.
            let mut total_owed: i128 = 0;
            for spec in winners.iter() {
                let percent = event.winner_distribution.get(spec.position).unwrap() as i128;
                let amount = event.total_budget.saturating_mul(percent) / 100_i128;
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
                let amount = event.total_budget.saturating_mul(percent) / 100_i128;

                escrow::release(env, &event.token, &spec.recipient, amount);
                event.remaining_escrow = event.remaining_escrow.saturating_sub(amount);

                // 1: bootstrap (idempotent on a fresh recipient).
                let bootstrap_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::BOOTSTRAP, sub_idx);
                profile.bootstrap(&spec.recipient, &bootstrap_op);

                // 2: earn credits.
                let earn_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::EARN_CREDITS, sub_idx);
                profile.earn_credits(
                    &spec.recipient,
                    &spec.credit_earn,
                    &reason_win,
                    &earn_op,
                );

                // 3: bump reputation.
                let rep_op =
                    idempotency::derive_child_indexed(env, &op_id, tag::BUMP_REP, sub_idx);
                profile.bump_reputation(
                    &spec.recipient,
                    &spec.reputation_bump,
                    &reason_win,
                    &rep_op,
                );

                // 4: register earnings in the event's token.
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

                winner_records.push_back(Winner {
                    recipient: spec.recipient.clone(),
                    position: spec.position,
                    amount,
                    milestone: None,
                    paid_at: Some(now),
                });

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
                winner_records.push_back(Winner {
                    recipient: spec.recipient.clone(),
                    position: spec.position,
                    amount: 0,
                    milestone: None,
                    paid_at: None,
                });
            }
        }
    }

    let winners_count = winners.len();
    storage::set_winners(env, event_id, &winner_records);
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
// ============================================================
pub fn get_event(env: &Env, event_id: u64) -> Result<EventRecord, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)
}

pub fn get_submission(env: &Env, event_id: u64, applicant: Address) -> Result<Submission, Error> {
    storage::get_submission(env, event_id, &applicant).ok_or(Error::SubmissionNotFound)
}

pub fn get_applicants(env: &Env, event_id: u64) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::get_applicants(env, event_id))
}

pub fn get_winners(env: &Env, event_id: u64) -> Result<Vec<Winner>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::get_winners(env, event_id))
}

pub fn get_contributors(env: &Env, event_id: u64) -> Result<Vec<Address>, Error> {
    storage::get_event(env, event_id).ok_or(Error::EventNotFound)?;
    Ok(storage::get_contributor_list(env, event_id))
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
