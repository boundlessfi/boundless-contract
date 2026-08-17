#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, BytesN, Env, Vec};

use soroban_sdk::String;

use crate::errors::Error;
use crate::types::{
    CancellationState, DataKey, EventRecord, PendingAdmin, PendingManager, PendingUpgrade,
    PrizeAward, Submission, Winner,
};

// ============================================================
// TTL CONSTANTS (mainnet cadence ~5s/ledger)
// ============================================================
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_BUMP: u32 = 518_400;

const EVENT_TTL_THRESHOLD: u32 = 86_400;
const EVENT_TTL_BUMP: u32 = 1_555_200;

#[contracttype(export = false)]
enum LegacyDataKey {
    OpSeen(BytesN<32>),
}

pub fn touch_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_BUMP);
}

fn touch_event_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, EVENT_TTL_THRESHOLD, EVENT_TTL_BUMP);
}

// ============================================================
// ADMIN / CONFIG (instance)
// ============================================================
pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_admin(env: &Env, addr: &Address) {
    env.storage().instance().set(&DataKey::Admin, addr);
}

pub fn get_pending_admin(env: &Env) -> Option<PendingAdmin> {
    env.storage().instance().get(&DataKey::PendingAdmin)
}

pub fn set_pending_admin(env: &Env, pending: &PendingAdmin) {
    env.storage()
        .instance()
        .set(&DataKey::PendingAdmin, pending);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingAdmin);
}

pub fn get_fee_account(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::FeeAccount)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_fee_account(env: &Env, addr: &Address) {
    env.storage().instance().set(&DataKey::FeeAccount, addr);
}

pub fn get_fee_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::FeeBps)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_fee_bps(env: &Env, bps: u32) {
    env.storage().instance().set(&DataKey::FeeBps, &bps);
}

pub fn get_profile_contract(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::ProfileContract)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, Error::NotInitialized))
}

pub fn set_profile_contract(env: &Env, addr: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::ProfileContract, addr);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

pub fn get_deployment_seq(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::DeploymentSeq)
        .unwrap_or(0)
}

pub fn set_deployment_seq(env: &Env, seq: u32) {
    env.storage().instance().set(&DataKey::DeploymentSeq, &seq);
}

// ============================================================
// VERSION / UPGRADE / MIGRATION (instance; H6)
// ============================================================
pub fn get_version(env: &Env) -> Option<String> {
    env.storage().instance().get(&DataKey::Version)
}

pub fn set_version(env: &Env, version: &String) {
    env.storage().instance().set(&DataKey::Version, version);
}

pub fn get_pending_upgrade(env: &Env) -> Option<PendingUpgrade> {
    env.storage().instance().get(&DataKey::PendingUpgrade)
}

pub fn set_pending_upgrade(env: &Env, pending: &PendingUpgrade) {
    env.storage()
        .instance()
        .set(&DataKey::PendingUpgrade, pending);
}

pub fn clear_pending_upgrade(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingUpgrade);
}

pub fn get_migrated_to_version(env: &Env) -> Option<String> {
    env.storage().instance().get(&DataKey::MigratedToVersion)
}

pub fn set_migrated_to_version(env: &Env, version: &String) {
    env.storage()
        .instance()
        .set(&DataKey::MigratedToVersion, version);
}

// ============================================================
// TOKEN WHITELIST (instance)
// ============================================================
pub fn is_token_supported(env: &Env, token: &Address) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::SupportedToken(token.clone()))
        .unwrap_or(false)
}

pub fn set_token_supported(env: &Env, token: &Address, supported: bool) {
    env.storage()
        .instance()
        .set(&DataKey::SupportedToken(token.clone()), &supported);
}

pub fn supported_token_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::SupportedTokenCount)
        .unwrap_or(0)
}

pub fn supported_token_at(env: &Env, idx: u32) -> Option<Address> {
    env.storage()
        .instance()
        .get(&DataKey::SupportedTokenAt(idx))
}

pub fn supported_token_slot(env: &Env, token: &Address) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::SupportedTokenSlot(token.clone()))
        .unwrap_or(0)
}

pub fn append_supported_token(env: &Env, token: &Address) {
    if supported_token_slot(env, token) != 0 {
        return;
    }
    let cur = supported_token_count(env);
    env.storage()
        .instance()
        .set(&DataKey::SupportedTokenAt(cur), token);
    let slot = cur.saturating_add(1);
    env.storage()
        .instance()
        .set(&DataKey::SupportedTokenSlot(token.clone()), &slot);
    env.storage()
        .instance()
        .set(&DataKey::SupportedTokenCount, &slot);
}

pub fn remove_supported_token(env: &Env, token: &Address) {
    let slot = supported_token_slot(env, token);
    if slot == 0 {
        return;
    }
    let idx = slot.saturating_sub(1);
    let count = supported_token_count(env);
    let last_idx = count.saturating_sub(1);

    if idx != last_idx {
        if let Some(last) = supported_token_at(env, last_idx) {
            env.storage()
                .instance()
                .set(&DataKey::SupportedTokenAt(idx), &last);
            env.storage()
                .instance()
                .set(&DataKey::SupportedTokenSlot(last), &slot);
        }
    }

    env.storage()
        .instance()
        .remove(&DataKey::SupportedTokenAt(last_idx));
    env.storage()
        .instance()
        .remove(&DataKey::SupportedTokenSlot(token.clone()));

    let new_count = count.saturating_sub(1);
    if new_count == 0 {
        env.storage()
            .instance()
            .remove(&DataKey::SupportedTokenCount);
    } else {
        env.storage()
            .instance()
            .set(&DataKey::SupportedTokenCount, &new_count);
    }
}

// ============================================================
// EVENT RECORD (persistent)
// ============================================================
pub fn get_next_event_id(env: &Env, fallback: u64) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextEventId)
        .unwrap_or(fallback)
}

pub fn set_next_event_id(env: &Env, id: u64) {
    env.storage().instance().set(&DataKey::NextEventId, &id);
}

pub fn get_event(env: &Env, id: u64) -> Option<EventRecord> {
    let key = DataKey::Event(id);
    let rec: Option<EventRecord> = env.storage().persistent().get(&key);
    if rec.is_some() {
        touch_event_persistent(env, &key);
    }
    rec
}

pub fn set_event(env: &Env, id: u64, record: &EventRecord) {
    let key = DataKey::Event(id);
    env.storage().persistent().set(&key, record);
    touch_event_persistent(env, &key);
}

pub fn get_event_manager(env: &Env, id: u64) -> Option<Address> {
    let key = DataKey::EventManager(id);
    let m: Option<Address> = env.storage().persistent().get(&key);
    if m.is_some() {
        touch_event_persistent(env, &key);
    }
    m
}

pub fn set_event_manager(env: &Env, id: u64, manager: &Address) {
    let key = DataKey::EventManager(id);
    env.storage().persistent().set(&key, manager);
    touch_event_persistent(env, &key);
}

pub fn get_pending_manager(env: &Env, id: u64) -> Option<PendingManager> {
    let key = DataKey::PendingManager(id);
    let p: Option<PendingManager> = env.storage().persistent().get(&key);
    if p.is_some() {
        touch_event_persistent(env, &key);
    }
    p
}

pub fn set_pending_manager(env: &Env, id: u64, pending: &PendingManager) {
    let key = DataKey::PendingManager(id);
    env.storage().persistent().set(&key, pending);
    touch_event_persistent(env, &key);
}

pub fn clear_pending_manager(env: &Env, id: u64) {
    env.storage()
        .persistent()
        .remove(&DataKey::PendingManager(id));
}

// ============================================================
// APPLICANTS (paged, persistent)
// ============================================================
pub fn applicant_count(env: &Env, id: u64) -> u32 {
    let key = DataKey::EventApplicantCount(id);
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

pub fn applicant_at(env: &Env, id: u64, idx: u32) -> Option<Address> {
    let key = DataKey::EventApplicantAt(id, idx);
    let addr: Option<Address> = env.storage().persistent().get(&key);
    if addr.is_some() {
        touch_event_persistent(env, &key);
    }
    addr
}

pub fn applicant_slot(env: &Env, id: u64, addr: &Address) -> u32 {
    let key = DataKey::EventApplicantSlot(id, addr.clone());
    let slot: Option<u32> = env.storage().persistent().get(&key);
    if slot.is_some() {
        touch_event_persistent(env, &key);
    }
    slot.unwrap_or(0)
}

pub fn append_applicant(env: &Env, id: u64, addr: &Address) -> Result<u32, Error> {
    if applicant_slot(env, id, addr) != 0 {
        return Err(Error::ApplicantAlreadyApplied);
    }
    let cur = applicant_count(env, id);
    // No product cap: each applicant is its own ledger entry paid for by the
    // applicant's own transaction, so growth is O(1) per append. Only guard
    // the u32 counter itself.
    let slot = cur.checked_add(1).ok_or(Error::TooManyApplicants)?;
    let at_key = DataKey::EventApplicantAt(id, cur);
    env.storage().persistent().set(&at_key, addr);
    touch_event_persistent(env, &at_key);

    let slot_key = DataKey::EventApplicantSlot(id, addr.clone());
    env.storage().persistent().set(&slot_key, &slot);
    touch_event_persistent(env, &slot_key);

    let count_key = DataKey::EventApplicantCount(id);
    env.storage().persistent().set(&count_key, &slot);
    touch_event_persistent(env, &count_key);
    Ok(slot)
}

pub fn remove_applicant(env: &Env, id: u64, addr: &Address) -> Result<(), Error> {
    let slot = applicant_slot(env, id, addr);
    if slot == 0 {
        return Err(Error::ApplicantNotApplied);
    }
    let idx = slot.checked_sub(1).ok_or(Error::ApplicantNotApplied)?;
    let count = applicant_count(env, id);
    let last_idx = count.checked_sub(1).ok_or(Error::EventNotFound)?;

    if idx != last_idx {
        let last_addr = applicant_at(env, id, last_idx).ok_or(Error::EventNotFound)?;
        let at_key = DataKey::EventApplicantAt(id, idx);
        env.storage().persistent().set(&at_key, &last_addr);
        touch_event_persistent(env, &at_key);

        let last_slot_key = DataKey::EventApplicantSlot(id, last_addr.clone());
        env.storage().persistent().set(&last_slot_key, &slot);
        touch_event_persistent(env, &last_slot_key);
    }

    env.storage()
        .persistent()
        .remove(&DataKey::EventApplicantAt(id, last_idx));
    env.storage()
        .persistent()
        .remove(&DataKey::EventApplicantSlot(id, addr.clone()));

    let count_key = DataKey::EventApplicantCount(id);
    let new_count = count.checked_sub(1).ok_or(Error::EventNotFound)?;
    if new_count == 0 {
        env.storage().persistent().remove(&count_key);
    } else {
        env.storage().persistent().set(&count_key, &new_count);
        touch_event_persistent(env, &count_key);
    }
    Ok(())
}

pub fn applicants_snapshot(env: &Env, id: u64, start: u32, limit: u32) -> Vec<Address> {
    let count = applicant_count(env, id);
    let end = start.saturating_add(limit).min(count);
    let mut out: Vec<Address> = Vec::new(env);
    for idx in start..end {
        if let Some(addr) = applicant_at(env, id, idx) {
            out.push_back(addr);
        }
    }
    out
}

// ============================================================
// SUBMISSIONS (per-entry, persistent)
// ============================================================
pub fn get_submission(env: &Env, id: u64, applicant: &Address, slot: u32) -> Option<Submission> {
    let key = DataKey::EventSubmissionEntry(id, applicant.clone(), slot);
    let s: Option<Submission> = env.storage().persistent().get(&key);
    if s.is_some() {
        touch_event_persistent(env, &key);
    }
    s
}

pub fn set_submission(env: &Env, id: u64, applicant: &Address, slot: u32, submission: &Submission) {
    let key = DataKey::EventSubmissionEntry(id, applicant.clone(), slot);
    env.storage().persistent().set(&key, submission);
    touch_event_persistent(env, &key);
}

/// How many slots this applicant currently occupies in this event. Drives the
/// O(1) "has this applicant submitted at all" check that gates application
/// withdrawal, which would otherwise have to scan slots.
pub fn applicant_submission_count(env: &Env, id: u64, applicant: &Address) -> u32 {
    let key = DataKey::EventApplicantSubmissionCount(id, applicant.clone());
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

pub fn has_any_submission(env: &Env, id: u64, applicant: &Address) -> bool {
    applicant_submission_count(env, id, applicant) > 0
}

fn set_applicant_submission_count(env: &Env, id: u64, applicant: &Address, count: u32) {
    let key = DataKey::EventApplicantSubmissionCount(id, applicant.clone());
    if count == 0 {
        env.storage().persistent().remove(&key);
        return;
    }
    env.storage().persistent().set(&key, &count);
    touch_event_persistent(env, &key);
}

pub fn remove_submission(env: &Env, id: u64, applicant: &Address, slot: u32) {
    // Idempotent: a no-op when the slot is empty, so a caller that skips its
    // own existence check cannot corrupt either counter.
    if get_submission(env, id, applicant, slot).is_none() {
        return;
    }

    let key = DataKey::EventSubmissionEntry(id, applicant.clone(), slot);
    env.storage().persistent().remove(&key);

    let per_applicant = applicant_submission_count(env, id, applicant).saturating_sub(1);
    set_applicant_submission_count(env, id, applicant, per_applicant);

    let count_key = DataKey::EventSubmissionCount(id);
    let next = submission_count(env, id).saturating_sub(1);
    if next == 0 {
        env.storage().persistent().remove(&count_key);
    } else {
        env.storage().persistent().set(&count_key, &next);
        touch_event_persistent(env, &count_key);
    }
}

pub fn submission_count(env: &Env, id: u64) -> u32 {
    let key = DataKey::EventSubmissionCount(id);
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

/// Count a newly occupied slot before writing it (mirrors
/// `append_contributor`/`append_applicant`). A no-op when the slot already
/// holds an entry — re-submitting to the same slot updates it in place and
/// must not recount.
///
/// Returns `Error::TooManyContributors` only on u32 counter overflow — reused
/// rather than a new variant since the errors enum is at the 50-case XDR cap.
pub fn append_submission(env: &Env, id: u64, addr: &Address, slot: u32) -> Result<(), Error> {
    if get_submission(env, id, addr, slot).is_some() {
        return Ok(());
    }
    let cur = submission_count(env, id);
    let next = cur.checked_add(1).ok_or(Error::TooManyContributors)?;
    let count_key = DataKey::EventSubmissionCount(id);
    env.storage().persistent().set(&count_key, &next);
    touch_event_persistent(env, &count_key);

    let per_applicant = applicant_submission_count(env, id, addr)
        .checked_add(1)
        .ok_or(Error::TooManyContributors)?;
    set_applicant_submission_count(env, id, addr, per_applicant);
    Ok(())
}

/// Reads a pre-1.7.0 submission row, which lived under a key with no slot.
/// Only `migrate` calls this.
pub fn get_legacy_submission(env: &Env, id: u64, applicant: &Address) -> Option<Submission> {
    env.storage()
        .persistent()
        .get(&DataKey::EventSubmission(id, applicant.clone()))
}

/// Drops a pre-1.7.0 submission row once it has been copied to a slot.
pub fn remove_legacy_submission(env: &Env, id: u64, applicant: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::EventSubmission(id, applicant.clone()));
}

/// Sets the per-applicant slot count directly. Only `migrate` calls this, to
/// seed the counter for rows that predate it.
pub fn seed_applicant_submission_count(env: &Env, id: u64, applicant: &Address, count: u32) {
    set_applicant_submission_count(env, id, applicant, count);
}

// ============================================================
// WINNERS (paged, persistent)
// ============================================================
pub fn winner_count(env: &Env, id: u64) -> u32 {
    let key = DataKey::EventWinnerCount(id);
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

pub fn winner_at(env: &Env, id: u64, idx: u32) -> Option<Winner> {
    let key = DataKey::EventWinnerAt(id, idx);
    let w: Option<Winner> = env.storage().persistent().get(&key);
    if w.is_some() {
        touch_event_persistent(env, &key);
    }
    w
}

pub fn append_winner(env: &Env, id: u64, w: &Winner) {
    let cur = winner_count(env, id);
    let at_key = DataKey::EventWinnerAt(id, cur);
    env.storage().persistent().set(&at_key, w);
    touch_event_persistent(env, &at_key);

    let count_key = DataKey::EventWinnerCount(id);
    let new_count = cur.saturating_add(1);
    env.storage().persistent().set(&count_key, &new_count);
    touch_event_persistent(env, &count_key);
}

pub fn set_winner_at(env: &Env, id: u64, idx: u32, w: &Winner) {
    let key = DataKey::EventWinnerAt(id, idx);
    env.storage().persistent().set(&key, w);
    touch_event_persistent(env, &key);
}

// ============================================================
// PRIZE AWARDS (pull-model claims; persistent, keyed by (event, position))
// ============================================================
pub fn get_prize_award(env: &Env, id: u64, position: u32) -> Option<PrizeAward> {
    let key = DataKey::EventPrizeAward(id, position);
    let a: Option<PrizeAward> = env.storage().persistent().get(&key);
    if a.is_some() {
        touch_event_persistent(env, &key);
    }
    a
}

pub fn set_prize_award(env: &Env, id: u64, position: u32, award: &PrizeAward) {
    let key = DataKey::EventPrizeAward(id, position);
    env.storage().persistent().set(&key, award);
    touch_event_persistent(env, &key);
}

pub fn unclaimed_prize_count(env: &Env, id: u64) -> u32 {
    let key = DataKey::EventUnclaimedPrizes(id);
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

pub fn set_unclaimed_prize_count(env: &Env, id: u64, count: u32) {
    let key = DataKey::EventUnclaimedPrizes(id);
    env.storage().persistent().set(&key, &count);
    touch_event_persistent(env, &key);
}

pub fn owed_total(env: &Env, id: u64) -> i128 {
    let key = DataKey::EventOwedTotal(id);
    let t: Option<i128> = env.storage().persistent().get(&key);
    if t.is_some() {
        touch_event_persistent(env, &key);
    }
    t.unwrap_or(0)
}

pub fn set_owed_total(env: &Env, id: u64, owed: i128) {
    let key = DataKey::EventOwedTotal(id);
    env.storage().persistent().set(&key, &owed);
    touch_event_persistent(env, &key);
}

pub fn get_prize_base_escrow(env: &Env, id: u64) -> Option<i128> {
    let key = DataKey::EventPrizeBaseEscrow(id);
    let b: Option<i128> = env.storage().persistent().get(&key);
    if b.is_some() {
        touch_event_persistent(env, &key);
    }
    b
}

pub fn set_prize_base_escrow(env: &Env, id: u64, base: i128) {
    let key = DataKey::EventPrizeBaseEscrow(id);
    env.storage().persistent().set(&key, &base);
    touch_event_persistent(env, &key);
}

pub fn get_prize_claim_expiry(env: &Env, id: u64) -> Option<u64> {
    let key = DataKey::EventPrizeClaimExpiry(id);
    let t: Option<u64> = env.storage().persistent().get(&key);
    if t.is_some() {
        touch_event_persistent(env, &key);
    }
    t
}

pub fn set_prize_claim_expiry(env: &Env, id: u64, expires_at: u64) {
    let key = DataKey::EventPrizeClaimExpiry(id);
    env.storage().persistent().set(&key, &expires_at);
    touch_event_persistent(env, &key);
}

pub fn winners_snapshot(env: &Env, id: u64, start: u32, limit: u32) -> Vec<Winner> {
    let count = winner_count(env, id);
    let end = start.saturating_add(limit).min(count);
    let mut out: Vec<Winner> = Vec::new(env);
    for idx in start..end {
        if let Some(w) = winner_at(env, id, idx) {
            out.push_back(w);
        }
    }
    out
}

// ============================================================
// CONTRIBUTIONS (paged, persistent)
// ============================================================
pub fn get_contributor_amount(env: &Env, id: u64, contributor: &Address) -> i128 {
    let key = DataKey::ContributorAmount(id, contributor.clone());
    let amt: Option<i128> = env.storage().persistent().get(&key);
    if amt.is_some() {
        touch_event_persistent(env, &key);
    }
    amt.unwrap_or(0_i128)
}

pub fn set_contributor_amount(env: &Env, id: u64, contributor: &Address, amount: i128) {
    let key = DataKey::ContributorAmount(id, contributor.clone());
    env.storage().persistent().set(&key, &amount);
    touch_event_persistent(env, &key);
}

pub fn get_non_owner_contribution_total(env: &Env, id: u64) -> Option<i128> {
    let key = DataKey::NonOwnerContributionTotal(id);
    let total: Option<i128> = env.storage().persistent().get(&key);
    if total.is_some() {
        touch_event_persistent(env, &key);
    }
    total
}

pub fn set_non_owner_contribution_total(env: &Env, id: u64, total: i128) {
    let key = DataKey::NonOwnerContributionTotal(id);
    env.storage().persistent().set(&key, &total);
    touch_event_persistent(env, &key);
}

pub fn contributor_count(env: &Env, id: u64) -> u32 {
    let key = DataKey::ContributorCount(id);
    let n: Option<u32> = env.storage().persistent().get(&key);
    if n.is_some() {
        touch_event_persistent(env, &key);
    }
    n.unwrap_or(0)
}

pub fn contributor_at(env: &Env, id: u64, idx: u32) -> Option<Address> {
    let key = DataKey::ContributorAt(id, idx);
    let addr: Option<Address> = env.storage().persistent().get(&key);
    if addr.is_some() {
        touch_event_persistent(env, &key);
    }
    addr
}

pub fn contributor_slot(env: &Env, id: u64, addr: &Address) -> u32 {
    let key = DataKey::ContributorSlot(id, addr.clone());
    let slot: Option<u32> = env.storage().persistent().get(&key);
    if slot.is_some() {
        touch_event_persistent(env, &key);
    }
    slot.unwrap_or(0)
}

pub fn append_contributor(env: &Env, id: u64, addr: &Address) -> Result<u32, Error> {
    if contributor_slot(env, id, addr) != 0 {
        return Ok(0);
    }
    let cur = contributor_count(env, id);
    // No product cap (see append_applicant); guard only the u32 counter.
    let slot = cur.checked_add(1).ok_or(Error::TooManyContributors)?;
    let at_key = DataKey::ContributorAt(id, cur);
    env.storage().persistent().set(&at_key, addr);
    touch_event_persistent(env, &at_key);

    let slot_key = DataKey::ContributorSlot(id, addr.clone());
    env.storage().persistent().set(&slot_key, &slot);
    touch_event_persistent(env, &slot_key);

    let count_key = DataKey::ContributorCount(id);
    env.storage().persistent().set(&count_key, &slot);
    touch_event_persistent(env, &count_key);
    Ok(slot)
}

pub fn contributors_snapshot(env: &Env, id: u64, start: u32, limit: u32) -> Vec<Address> {
    let count = contributor_count(env, id);
    let end = start.saturating_add(limit).min(count);
    let mut out: Vec<Address> = Vec::new(env);
    for idx in start..end {
        if let Some(addr) = contributor_at(env, id, idx) {
            out.push_back(addr);
        }
    }
    out
}

// ============================================================
// GRANT MILESTONES (persistent)
// ============================================================
pub fn is_milestone_claimed(env: &Env, id: u64, recipient: &Address, milestone: u32) -> bool {
    let key = DataKey::MilestoneClaimed(id, recipient.clone(), milestone);
    let claimed: Option<bool> = env.storage().persistent().get(&key);
    if claimed.is_some() {
        touch_event_persistent(env, &key);
    }
    claimed.unwrap_or(false)
}

pub fn mark_milestone_claimed(env: &Env, id: u64, recipient: &Address, milestone: u32) {
    let key = DataKey::MilestoneClaimed(id, recipient.clone(), milestone);
    env.storage().persistent().set(&key, &true);
    touch_event_persistent(env, &key);
}

// ============================================================
// CROWDFUNDING MILESTONES CLAIMED (persistent)
// ============================================================
pub fn get_crowdfunding_milestones_claimed(env: &Env, id: u64) -> u32 {
    let key = DataKey::CrowdfundingMilestonesClaimed(id);
    let claimed: Option<u32> = env.storage().persistent().get(&key);
    if claimed.is_some() {
        touch_event_persistent(env, &key);
    }
    claimed.unwrap_or(0_u32)
}

pub fn set_crowdfunding_milestones_claimed(env: &Env, id: u64, count: u32) {
    let key = DataKey::CrowdfundingMilestonesClaimed(id);
    env.storage().persistent().set(&key, &count);
    touch_event_persistent(env, &key);
}

// ============================================================
// CANCELLATION STATE (persistent; present only while Cancelling)
// ============================================================
pub fn get_cancellation_state(env: &Env, id: u64) -> Option<CancellationState> {
    let key = DataKey::CancellationState(id);
    let s: Option<CancellationState> = env.storage().persistent().get(&key);
    if s.is_some() {
        touch_event_persistent(env, &key);
    }
    s
}

pub fn set_cancellation_state(env: &Env, id: u64, state: &CancellationState) {
    let key = DataKey::CancellationState(id);
    env.storage().persistent().set(&key, state);
    touch_event_persistent(env, &key);
}

pub fn clear_cancellation_state(env: &Env, id: u64) {
    env.storage()
        .persistent()
        .remove(&DataKey::CancellationState(id));
}

// ============================================================
// IDEMPOTENCY (temporary; auto-TTL)
// ============================================================
pub fn is_op_seen(env: &Env, domain: &Address, op_id: &BytesN<32>) -> bool {
    let scoped_seen = env
        .storage()
        .temporary()
        .get(&DataKey::OpSeen(domain.clone(), op_id.clone()))
        .unwrap_or(false);
    scoped_seen
        || env
            .storage()
            .temporary()
            .get(&LegacyDataKey::OpSeen(op_id.clone()))
            .unwrap_or(false)
}

pub fn mark_op_seen(env: &Env, domain: &Address, op_id: &BytesN<32>) {
    env.storage()
        .temporary()
        .set(&DataKey::OpSeen(domain.clone(), op_id.clone()), &true);
}
