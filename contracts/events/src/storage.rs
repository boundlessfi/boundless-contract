// boundless-events: storage helpers.
//
// Persistent storage for everything except OpSeen (which uses temporary
// storage so it can expire). DataKey variants from types.rs are the canonical
// key set; helpers here keep the call sites readable.
//
// dead_code allowed at the module level: many setters are referenced only by
// operation bodies that are stubbed in the scaffolding pass. Wired up as those
// bodies land.
#![allow(dead_code)]

use soroban_sdk::{Address, BytesN, Env, Vec};

use crate::errors::Error;
use crate::types::{DataKey, EventRecord, PendingAdmin, Submission, Winner};

// ============================================================
// ADMIN / CONFIG (persistent)
// ============================================================
pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(Error::AlreadyInitialized)
}

pub fn set_admin(env: &Env, addr: &Address) {
    env.storage().persistent().set(&DataKey::Admin, addr);
}

pub fn get_pending_admin(env: &Env) -> Option<PendingAdmin> {
    env.storage().persistent().get(&DataKey::PendingAdmin)
}

pub fn set_pending_admin(env: &Env, pending: &PendingAdmin) {
    env.storage()
        .persistent()
        .set(&DataKey::PendingAdmin, pending);
}

pub fn clear_pending_admin(env: &Env) {
    env.storage().persistent().remove(&DataKey::PendingAdmin);
}

pub fn get_fee_account(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::FeeAccount)
        .expect("fee account not configured")
}

pub fn set_fee_account(env: &Env, addr: &Address) {
    env.storage().persistent().set(&DataKey::FeeAccount, addr);
}

pub fn get_fee_bps(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::FeeBps)
        .expect("fee bps not configured")
}

pub fn set_fee_bps(env: &Env, bps: u32) {
    env.storage().persistent().set(&DataKey::FeeBps, &bps);
}

pub fn get_profile_contract(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::ProfileContract)
        .expect("profile contract not configured")
}

pub fn set_profile_contract(env: &Env, addr: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::ProfileContract, addr);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().persistent().set(&DataKey::Paused, &paused);
}

pub fn get_deployment_seq(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::DeploymentSeq)
        .unwrap_or(0)
}

pub fn set_deployment_seq(env: &Env, seq: u32) {
    env.storage().persistent().set(&DataKey::DeploymentSeq, &seq);
}

// ============================================================
// TOKEN WHITELIST (persistent)
// ============================================================
pub fn is_token_supported(env: &Env, token: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedToken(token.clone()))
        .unwrap_or(false)
}

pub fn set_token_supported(env: &Env, token: &Address, supported: bool) {
    env.storage()
        .persistent()
        .set(&DataKey::SupportedToken(token.clone()), &supported);
}

// ============================================================
// EVENT RECORD (persistent)
// ============================================================
pub fn get_next_event_id(env: &Env, fallback: u64) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::NextEventId)
        .unwrap_or(fallback)
}

pub fn set_next_event_id(env: &Env, id: u64) {
    env.storage().persistent().set(&DataKey::NextEventId, &id);
}

pub fn get_event(env: &Env, id: u64) -> Option<EventRecord> {
    env.storage().persistent().get(&DataKey::Event(id))
}

pub fn set_event(env: &Env, id: u64, record: &EventRecord) {
    env.storage().persistent().set(&DataKey::Event(id), record);
}

pub fn get_applicants(env: &Env, id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::EventApplicants(id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_applicants(env: &Env, id: u64, applicants: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::EventApplicants(id), applicants);
}

pub fn get_submission(env: &Env, id: u64, applicant: &Address) -> Option<Submission> {
    let map: Option<soroban_sdk::Map<Address, Submission>> = env
        .storage()
        .persistent()
        .get(&DataKey::EventSubmissions(id));
    map.and_then(|m| m.get(applicant.clone()))
}

pub fn set_submission(env: &Env, id: u64, applicant: &Address, submission: &Submission) {
    let mut map: soroban_sdk::Map<Address, Submission> = env
        .storage()
        .persistent()
        .get(&DataKey::EventSubmissions(id))
        .unwrap_or_else(|| soroban_sdk::Map::new(env));
    map.set(applicant.clone(), submission.clone());
    env.storage()
        .persistent()
        .set(&DataKey::EventSubmissions(id), &map);
}

pub fn remove_submission(env: &Env, id: u64, applicant: &Address) {
    let mut map: soroban_sdk::Map<Address, Submission> = env
        .storage()
        .persistent()
        .get(&DataKey::EventSubmissions(id))
        .unwrap_or_else(|| soroban_sdk::Map::new(env));
    map.remove(applicant.clone());
    env.storage()
        .persistent()
        .set(&DataKey::EventSubmissions(id), &map);
}

pub fn get_winners(env: &Env, id: u64) -> Vec<Winner> {
    env.storage()
        .persistent()
        .get(&DataKey::EventWinners(id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_winners(env: &Env, id: u64, winners: &Vec<Winner>) {
    env.storage()
        .persistent()
        .set(&DataKey::EventWinners(id), winners);
}

// ============================================================
// CONTRIBUTIONS (persistent)
//
// Two keys per event:
//   * ContributorAmount(event_id, addr) -> i128 running total
//   * ContributorList(event_id)         -> Vec<Address> deposit order
//
// cancel_event walks ContributorList in order, looks up the running total
// for each entry, and pays out. Owner deposits live in event.total_budget
// and are NOT recorded here.
// ============================================================
pub fn get_contributor_amount(env: &Env, id: u64, contributor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::ContributorAmount(id, contributor.clone()))
        .unwrap_or(0_i128)
}

pub fn set_contributor_amount(env: &Env, id: u64, contributor: &Address, amount: i128) {
    env.storage().persistent().set(
        &DataKey::ContributorAmount(id, contributor.clone()),
        &amount,
    );
}

pub fn get_contributor_list(env: &Env, id: u64) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::ContributorList(id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_contributor_list(env: &Env, id: u64, list: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::ContributorList(id), list);
}

// ============================================================
// GRANT MILESTONES (persistent)
// ============================================================
pub fn is_milestone_claimed(env: &Env, id: u64, recipient: &Address, milestone: u32) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::MilestoneClaimed(id, recipient.clone(), milestone))
        .unwrap_or(false)
}

pub fn mark_milestone_claimed(env: &Env, id: u64, recipient: &Address, milestone: u32) {
    env.storage().persistent().set(
        &DataKey::MilestoneClaimed(id, recipient.clone(), milestone),
        &true,
    );
}

// ============================================================
// CROWDFUNDING MILESTONES CLAIMED (persistent)
//
// Crowdfunding's claim_milestone uses dynamic math:
//   amount = remaining_escrow / (total_milestones - claimed_so_far)
//
// We need a counter so each call sees how many milestones have already
// been paid. The key is event-scoped (not recipient-scoped) because
// crowdfunding has exactly one recipient per event by construction.
// ============================================================
pub fn get_crowdfunding_milestones_claimed(env: &Env, id: u64) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::CrowdfundingMilestonesClaimed(id))
        .unwrap_or(0_u32)
}

pub fn set_crowdfunding_milestones_claimed(env: &Env, id: u64, count: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::CrowdfundingMilestonesClaimed(id), &count);
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
