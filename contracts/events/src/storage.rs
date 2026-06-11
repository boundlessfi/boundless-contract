// boundless-events: storage helpers.
//
// Storage layout (after the 2026-06 audit, H1-H4):
//
//   instance()    — admin + config + token whitelist + NextEventId.
//   persistent()  — per-event Event records, per-event paged lists
//                   (applicants, contributors, winners), and per-submission
//                   entries. Each persistent read/write bumps TTL.
//   temporary()   — OpSeen idempotency markers.
//
// Per-event lists are kept as count + indexed-entry pairs (e.g.
// EventApplicantCount + EventApplicantAt(idx) + EventApplicantSlot(addr))
// so a single growable Vec never overflows the 64KB ledger-entry cap and
// presence checks stay O(1).
//
// Cap policy (event_ops::MAX_*_PER_EVENT) enforced at append time so that
// cancel_event refund passes stay inside Soroban's per-tx footprint budget.
// Paging cancel_event is a P1 follow-up before lifting the caps.

#![allow(dead_code)]

use soroban_sdk::{Address, BytesN, Env, Vec};

use soroban_sdk::String;

use crate::errors::Error;
use crate::types::{
    CancellationState, DataKey, EventRecord, PendingAdmin, PendingUpgrade, Submission, Winner,
};

// ============================================================
// TTL CONSTANTS (mainnet cadence ~5s/ledger)
// ============================================================
const INSTANCE_TTL_THRESHOLD: u32 = 17_280;
const INSTANCE_TTL_BUMP: u32 = 518_400;

const EVENT_TTL_THRESHOLD: u32 = 86_400;
const EVENT_TTL_BUMP: u32 = 1_555_200;

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

// Per-event management authority override (side map; absent => owner manages).
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

// ============================================================
// APPLICANTS (paged, persistent)
//
// applicant_count(id)        -> number of applicants in [0, count).
// applicant_at(id, idx)      -> address at slot idx (Some only when idx < count).
// applicant_slot(id, addr)   -> 1-based slot. 0 means absent. Stored 1-based
//                               so a missing key (default 0) signals absence
//                               without an Option round trip.
// append_applicant(id, addr) -> appends to the tail. Returns the 1-based slot.
//                               Fails with TooManyApplicants if cap is hit.
// remove_applicant(id, addr) -> swap-with-last + decrement.
// applicants_snapshot(id, max) -> Vec view capped at `max`; older callers
//                                 that read the whole list should migrate
//                                 to paged reads.
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

pub fn append_applicant(env: &Env, id: u64, addr: &Address, cap: u32) -> Result<u32, Error> {
    if applicant_slot(env, id, addr) != 0 {
        return Err(Error::ApplicantAlreadyApplied);
    }
    let cur = applicant_count(env, id);
    if cur >= cap {
        return Err(Error::TooManyApplicants);
    }
    let at_key = DataKey::EventApplicantAt(id, cur);
    env.storage().persistent().set(&at_key, addr);
    touch_event_persistent(env, &at_key);

    let slot_key = DataKey::EventApplicantSlot(id, addr.clone());
    let slot = cur.saturating_add(1);
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
    let idx = slot - 1;
    let count = applicant_count(env, id);
    let last_idx = count - 1;

    // If not the last entry, swap the last applicant into the freed slot.
    if idx != last_idx {
        let last_addr = applicant_at(env, id, last_idx).expect("count > 0 implies last present");
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
    let new_count = count - 1;
    if new_count == 0 {
        env.storage().persistent().remove(&count_key);
    } else {
        env.storage().persistent().set(&count_key, &new_count);
        touch_event_persistent(env, &count_key);
    }
    Ok(())
}

pub fn applicants_snapshot(env: &Env, id: u64, max: u32) -> Vec<Address> {
    let count = applicant_count(env, id);
    let upper = if count < max { count } else { max };
    let mut out: Vec<Address> = Vec::new(env);
    for idx in 0..upper {
        if let Some(addr) = applicant_at(env, id, idx) {
            out.push_back(addr);
        }
    }
    out
}

// ============================================================
// SUBMISSIONS (per-entry, persistent)
// ============================================================
pub fn get_submission(env: &Env, id: u64, applicant: &Address) -> Option<Submission> {
    let key = DataKey::EventSubmission(id, applicant.clone());
    let s: Option<Submission> = env.storage().persistent().get(&key);
    if s.is_some() {
        touch_event_persistent(env, &key);
    }
    s
}

pub fn set_submission(env: &Env, id: u64, applicant: &Address, submission: &Submission) {
    let key = DataKey::EventSubmission(id, applicant.clone());
    env.storage().persistent().set(&key, submission);
    touch_event_persistent(env, &key);
}

pub fn remove_submission(env: &Env, id: u64, applicant: &Address) {
    let key = DataKey::EventSubmission(id, applicant.clone());
    env.storage().persistent().remove(&key);
}

// ============================================================
// WINNERS (paged, persistent)
//
// winner_count(id)         -> number of winner rows (anchors + per-milestone).
// winner_at(id, idx)       -> Winner at slot idx.
// append_winner(id, w)     -> append-only; select_winners and claim_milestone
//                             never rewrite existing rows.
// winners_snapshot(id,max) -> Vec view capped at `max`. Callers needing full
//                             iteration should use the paged API.
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

pub fn winners_snapshot(env: &Env, id: u64, max: u32) -> Vec<Winner> {
    let count = winner_count(env, id);
    let upper = if count < max { count } else { max };
    let mut out: Vec<Winner> = Vec::new(env);
    for idx in 0..upper {
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

pub fn append_contributor(env: &Env, id: u64, addr: &Address, cap: u32) -> Result<u32, Error> {
    if contributor_slot(env, id, addr) != 0 {
        return Ok(0); // already present; caller treats 0 as "no-op"
    }
    let cur = contributor_count(env, id);
    if cur >= cap {
        return Err(Error::TooManyContributors);
    }
    let at_key = DataKey::ContributorAt(id, cur);
    env.storage().persistent().set(&at_key, addr);
    touch_event_persistent(env, &at_key);

    let slot_key = DataKey::ContributorSlot(id, addr.clone());
    let slot = cur.saturating_add(1);
    env.storage().persistent().set(&slot_key, &slot);
    touch_event_persistent(env, &slot_key);

    let count_key = DataKey::ContributorCount(id);
    env.storage().persistent().set(&count_key, &slot);
    touch_event_persistent(env, &count_key);
    Ok(slot)
}

pub fn contributors_snapshot(env: &Env, id: u64, max: u32) -> Vec<Address> {
    let count = contributor_count(env, id);
    let upper = if count < max { count } else { max };
    let mut out: Vec<Address> = Vec::new(env);
    for idx in 0..upper {
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
pub fn is_op_seen(env: &Env, op_id: &BytesN<32>) -> bool {
    env.storage()
        .temporary()
        .get(&DataKey::OpSeen(op_id.clone()))
        .unwrap_or(false)
}

pub fn mark_op_seen(env: &Env, op_id: &BytesN<32>) {
    env.storage()
        .temporary()
        .set(&DataKey::OpSeen(op_id.clone()), &true);
}
