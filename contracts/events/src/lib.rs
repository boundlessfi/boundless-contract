// SPDX-License-Identifier: MIT
//
// boundless-events
//
// On-chain event records (Hackathon, Bounty, Grant) plus inlined escrow.
// Companion: boundless-profile (credits + reputation).
// Spec: boundless-platform-contract-prd.md
#![no_std]

use soroban_sdk::{contract, contractimpl, contractmeta, Address, BytesN, Env, String, Vec};

mod admin;
mod bounty;
mod crowdfunding;
mod errors;
mod escrow;
mod event_ops;
mod events;
mod grant;
mod hackathon;
mod idempotency;
mod profile_client;
mod storage;
mod token_whitelist;
mod types;

#[cfg(test)]
mod tests;

use crate::errors::Error;
use crate::types::*;

contractmeta!(key = "version", val = "0.1.0");
contractmeta!(
    key = "description",
    val = "Boundless events contract: hackathon, bounty, grant + escrow"
);
contractmeta!(key = "license", val = "MIT");

#[contract]
pub struct EventsContract;

#[contractimpl]
impl EventsContract {
    // ============================================================
    // CONSTRUCTOR
    // ============================================================
    pub fn __constructor(
        env: Env,
        admin: Address,
        fee_account: Address,
        fee_bps: u32,
        profile_contract: Address,
    ) {
        admin::initialize(&env, admin, fee_account, fee_bps, profile_contract);
    }

    // ============================================================
    // ADMIN
    // ============================================================
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        admin::set_admin(&env, new_admin)
    }

    pub fn accept_admin(env: Env) -> Result<(), Error> {
        admin::accept_admin(&env)
    }

    pub fn set_fee_bps(env: Env, new_bps: u32) -> Result<(), Error> {
        admin::set_fee_bps(&env, new_bps)
    }

    pub fn set_fee_account(env: Env, new_account: Address) -> Result<(), Error> {
        admin::set_fee_account(&env, new_account)
    }

    pub fn set_profile_contract(env: Env, new_addr: Address) -> Result<(), Error> {
        admin::set_profile_contract(&env, new_addr)
    }

    pub fn pause(env: Env) -> Result<(), Error> {
        admin::pause(&env)
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        admin::unpause(&env)
    }

    pub fn propose_upgrade(
        env: Env,
        new_wasm_hash: BytesN<32>,
        new_version: String,
    ) -> Result<(), Error> {
        admin::propose_upgrade(&env, new_wasm_hash, new_version)
    }

    pub fn apply_upgrade(env: Env) -> Result<(), Error> {
        admin::apply_upgrade(&env)
    }

    pub fn cancel_pending_upgrade(env: Env) -> Result<(), Error> {
        admin::cancel_pending_upgrade(&env)
    }

    pub fn migrate(env: Env) -> Result<(), Error> {
        admin::migrate(&env)
    }

    // ============================================================
    // TOKEN WHITELIST
    // ============================================================
    pub fn register_supported_token(env: Env, token: Address) -> Result<(), Error> {
        token_whitelist::register(&env, token)
    }

    pub fn deregister_supported_token(env: Env, token: Address) -> Result<(), Error> {
        token_whitelist::deregister(&env, token)
    }

    pub fn is_supported_token(env: Env, token: Address) -> bool {
        token_whitelist::is_supported(&env, &token)
    }

    // ============================================================
    // EVENT LIFECYCLE
    // ============================================================
    pub fn create_event(
        env: Env,
        params: CreateEventParams,
        op_id: BytesN<32>,
    ) -> Result<u64, Error> {
        event_ops::create_event(&env, params, op_id)
    }

    pub fn start_cancel(env: Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
        event_ops::start_cancel(&env, event_id, op_id)
    }

    pub fn process_cancel_batch(
        env: Env,
        event_id: u64,
        max_refunds: u32,
        op_id: BytesN<32>,
    ) -> Result<u32, Error> {
        event_ops::process_cancel_batch(&env, event_id, max_refunds, op_id)
    }

    pub fn finalize_cancel(env: Env, event_id: u64, op_id: BytesN<32>) -> Result<(), Error> {
        event_ops::finalize_cancel(&env, event_id, op_id)
    }

    pub fn add_funds(
        env: Env,
        event_id: u64,
        from: Address,
        amount: i128,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        event_ops::add_funds(&env, event_id, from, amount, op_id)
    }

    // ============================================================
    // BOUNTY PARTICIPATION
    // ============================================================
    pub fn apply_to_bounty(
        env: Env,
        bounty_id: u64,
        applicant: Address,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        bounty::apply(&env, bounty_id, applicant, op_id)
    }

    pub fn withdraw_application(
        env: Env,
        bounty_id: u64,
        applicant: Address,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        bounty::withdraw_application(&env, bounty_id, applicant, op_id)
    }

    // ============================================================
    // SUBMISSION
    // ============================================================
    pub fn submit(
        env: Env,
        event_id: u64,
        applicant: Address,
        content_uri: String,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        event_ops::submit(&env, event_id, applicant, content_uri, op_id)
    }

    pub fn withdraw_submission(
        env: Env,
        event_id: u64,
        applicant: Address,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        event_ops::withdraw_submission(&env, event_id, applicant, op_id)
    }

    // ============================================================
    // WINNERS
    // ============================================================
    pub fn select_winners(
        env: Env,
        event_id: u64,
        winners: Vec<WinnerSpec>,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        event_ops::select_winners(&env, event_id, winners, op_id)
    }

    pub fn claim_milestone(
        env: Env,
        event_id: u64,
        recipient: Address,
        milestone: u32,
        credit_earn: u32,
        reputation_bump: u32,
        op_id: BytesN<32>,
    ) -> Result<(), Error> {
        grant::claim_milestone(
            &env,
            event_id,
            recipient,
            milestone,
            credit_earn,
            reputation_bump,
            op_id,
        )
    }

    // ============================================================
    // READS (id-keyed only; no linear scans)
    // ============================================================
    pub fn get_event(env: Env, event_id: u64) -> Result<EventRecord, Error> {
        event_ops::get_event(&env, event_id)
    }

    pub fn get_submission(
        env: Env,
        event_id: u64,
        applicant: Address,
    ) -> Result<Submission, Error> {
        event_ops::get_submission(&env, event_id, applicant)
    }

    pub fn get_applicants(env: Env, event_id: u64) -> Result<Vec<Address>, Error> {
        event_ops::get_applicants(&env, event_id)
    }

    pub fn get_applicant_count(env: Env, event_id: u64) -> Result<u32, Error> {
        event_ops::get_applicant_count(&env, event_id)
    }

    pub fn get_applicant_at(env: Env, event_id: u64, idx: u32) -> Result<Option<Address>, Error> {
        event_ops::get_applicant_at(&env, event_id, idx)
    }

    pub fn get_winners(env: Env, event_id: u64) -> Result<Vec<Winner>, Error> {
        event_ops::get_winners(&env, event_id)
    }

    pub fn get_winner_count(env: Env, event_id: u64) -> Result<u32, Error> {
        event_ops::get_winner_count(&env, event_id)
    }

    pub fn get_winner_at(env: Env, event_id: u64, idx: u32) -> Result<Option<Winner>, Error> {
        event_ops::get_winner_at(&env, event_id, idx)
    }

    pub fn get_contributors(env: Env, event_id: u64) -> Result<Vec<Address>, Error> {
        event_ops::get_contributors(&env, event_id)
    }

    pub fn get_contributor_count(env: Env, event_id: u64) -> Result<u32, Error> {
        event_ops::get_contributor_count(&env, event_id)
    }

    pub fn get_contributor_at(env: Env, event_id: u64, idx: u32) -> Result<Option<Address>, Error> {
        event_ops::get_contributor_at(&env, event_id, idx)
    }

    pub fn get_contributor_amount(
        env: Env,
        event_id: u64,
        contributor: Address,
    ) -> Result<i128, Error> {
        event_ops::get_contributor_amount(&env, event_id, contributor)
    }

    pub fn get_admin(env: Env) -> Address {
        admin::get_admin(&env)
    }

    pub fn get_fee_bps(env: Env) -> u32 {
        admin::get_fee_bps(&env)
    }

    pub fn get_fee_account(env: Env) -> Address {
        admin::get_fee_account(&env)
    }

    pub fn get_profile_contract(env: Env) -> Address {
        admin::get_profile_contract(&env)
    }

    pub fn is_paused(env: Env) -> bool {
        admin::is_paused(&env)
    }

    pub fn version(env: Env) -> String {
        admin::get_version(&env)
    }

    pub fn get_pending_upgrade(env: Env) -> Option<PendingUpgrade> {
        admin::get_pending_upgrade(&env)
    }

    pub fn get_migrated_to_version(env: Env) -> Option<String> {
        admin::get_migrated_to_version(&env)
    }

    // Internal helper exposed for off-chain inspection; emits no event.
    pub fn id_base(env: Env) -> u64 {
        idempotency::id_base(&env)
    }
}
