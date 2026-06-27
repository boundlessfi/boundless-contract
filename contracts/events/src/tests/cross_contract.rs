// boundless-events: cross-contract integration test.
//
// Deploys boundless-events + a real boundless-profile, wires them together,
// and exercises cross-contract flows (select_winners, submit, cancel, grants).
// Bounty apply / withdraw coverage lives in tests/bounty_pillar.rs.
//
// Spec: boundless-platform-contract-prd.md Section 4; boundless-credits-reputation-prd.md Section 10.1.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

// boundless-profile lives in its own crate. For the integration test we use
// the WASM artifact path that Soroban's testutils supports. Since we are in a
// host-target test (not wasm32v1-none), we import the profile contract crate
// directly and register it.
use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    profile: ProfileContractClient<'a>,
    owner: Address,
    applicant: Address,
    token_addr: Address,
    fee_account: Address,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    // Auth from non-root contract invocations is required (token transfers in
    // create_event, cross-contract calls into the profile contract).
    env.mock_all_auths_allowing_non_root_auth();

    // Deploy profile contract.
    let profile_admin = Address::generate(&env);
    let profile_id = env.register(
        ProfileContract,
        (profile_admin.clone(), BOOTSTRAP_CREDITS),
    );
    let profile = ProfileContractClient::new(&env, &profile_id);

    // Deploy events contract pointing at profile.
    let events_admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (
            events_admin.clone(),
            fee_account.clone(),
            FEE_BPS,
            profile_id.clone(),
        ),
    );
    let events = EventsContractClient::new(&env, &events_id);

    // Wire profile to recognize the events contract.
    profile.set_events_contract(&events_id);

    // Mock USDC via Stellar Asset Contract.
    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let token_addr = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_addr);

    // Touch fee_account's trustline (mint 0).
    token_admin.mint(&fee_account, &0);

    // Mint USDC to the bounty owner.
    let owner = Address::generate(&env);
    token_admin.mint(&owner, &1_000_000_0000000_i128);

    // Register USDC on events.
    events.register_supported_token(&token_addr);

    let applicant = Address::generate(&env);

    Ctx {
        env,
        events,
        profile,
        owner,
        applicant,
        token_addr,
        fee_account,
    }
}

fn one_winner_distribution(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_bounty(ctx: &Ctx, application_credit_cost: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: 10_000_0000000_i128, // 10k USDC at 7 decimals
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/events/draft/x"),
        title: String::from_str(&ctx.env, "Test Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost,
        fee_bps_override: None,
        manager: None,
    };
    let op_id = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_id)
}

// ============================================================
// select_winners
// ============================================================

const TOTAL_BUDGET: i128 = 10_000_0000000_i128;
const FEE_AMOUNT: i128 = (TOTAL_BUDGET * FEE_BPS as i128) / 10_000_i128;

#[test]
fn select_winners_pays_recipient_and_bumps_profile() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    // Applicant applies, getting bootstrapped.
    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    // Owner picks applicant as the sole winner of position 1 (100% of budget).
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Token: winner received the full budget (no second-layer fee on release).
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET);
    // Fee account got the deposit-time fee.
    assert_eq!(token.balance(&ctx.fee_account), FEE_AMOUNT);

    // Profile: credits = 10 (bootstrap) - 1 (apply) + 20 (win) = 29.
    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 1 + 20);
    assert_eq!(profile.reputation, 50);

    // Earnings registered against the event's token.
    let earnings = ctx
        .profile
        .get_earnings(&ctx.applicant, &ctx.token_addr);
    assert_eq!(earnings, TOTAL_BUDGET);

    // Event completed.
    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);

    // Winner record stored.
    let winner_list = ctx.events.get_winners(&bounty_id);
    assert_eq!(winner_list.len(), 1);
    let recorded = winner_list.get(0).unwrap();
    assert_eq!(recorded.recipient, ctx.applicant);
    assert_eq!(recorded.position, 1);
    assert_eq!(recorded.amount, TOTAL_BUDGET);
    assert_eq!(recorded.milestone, None);
    assert!(recorded.paid_at.is_some());
}

#[test]
fn select_winners_requires_position_in_distribution() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 2, // distribution only has position 1
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_select_winners(&bounty_id, &winners, &op_select);
    assert!(res.is_err(), "invalid position should revert");
}

#[test]
fn select_winners_rejects_duplicate_position() {
    let ctx = setup();
    // Use a distribution split 60/40 across positions 1 and 2 so the test
    // can supply a duplicate position 1 entry.
    let owner = ctx.owner.clone();
    let token_addr = ctx.token_addr.clone();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner,
        token: token_addr,
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/x"),
        title: String::from_str(&ctx.env, "Test Bounty 2"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 1,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let bounty_id = ctx.events.create_event(&params, &op_create);

    let other_recipient = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
        WinnerSpec {
            recipient: other_recipient,
            position: 1, // duplicate
            credit_earn: 10,
            reputation_bump: 25,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_select_winners(&bounty_id, &winners, &op_select);
    assert!(res.is_err(), "duplicate position should revert");
}

#[test]
fn select_winners_handles_multi_recipient_distribution() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/multi"),
        title: String::from_str(&ctx.env, "Multi Winner"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0, // free for this test
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let bounty_id = ctx.events.create_event(&params, &op_create);

    let winner_a = Address::generate(&ctx.env);
    let winner_b = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: winner_a.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
        WinnerSpec {
            recipient: winner_b.clone(),
            position: 2,
            credit_earn: 10,
            reputation_bump: 25,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let amount_a = TOTAL_BUDGET * 60 / 100;
    let amount_b = TOTAL_BUDGET * 40 / 100;
    assert_eq!(token.balance(&winner_a), amount_a);
    assert_eq!(token.balance(&winner_b), amount_b);

    let profile_a = ctx.profile.get_profile(&winner_a).unwrap();
    let profile_b = ctx.profile.get_profile(&winner_b).unwrap();
    assert_eq!(profile_a.credits, BOOTSTRAP_CREDITS + 20);
    assert_eq!(profile_a.reputation, 50);
    assert_eq!(profile_b.credits, BOOTSTRAP_CREDITS + 10);
    assert_eq!(profile_b.reputation, 25);

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn select_winners_replayed_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let res = ctx
        .events
        .try_select_winners(&bounty_id, &winners, &op_select);
    assert!(res.is_err(), "replayed select_winners should revert");
}

// ============================================================
// cancel_event
// ============================================================

#[test]
fn cancel_refunds_remaining_escrow_to_owner() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, bounty_id);

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);

    let owner_after = token.balance(&ctx.owner);
    assert_eq!(owner_after - owner_before, TOTAL_BUDGET);
}

#[test]
fn cancel_already_cancelled_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    drive_cancel(&ctx.env, &ctx.events, bounty_id);

    // Second start_cancel on a Cancelled event must revert.
    let op_again = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_start_cancel(&bounty_id, &op_again);
    assert!(res.is_err(), "second cancel should revert");
}

#[test]
fn cancel_after_select_winners_refunds_only_remaining() {
    let ctx = setup();
    // 60/40 split so we can pay one winner and then cancel.
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/partial"),
        title: String::from_str(&ctx.env, "Partial Pay Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let bounty_id = ctx.events.create_event(&params, &op_create);

    // Pay one winner (60%).
    let winner_a = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: winner_a.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Now cancel — owner should get the remaining 40%.
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, bounty_id);

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);

    let owner_after = token.balance(&ctx.owner);
    assert_eq!(owner_after - owner_before, TOTAL_BUDGET * 40 / 100);
}

// ============================================================
// claim_milestone
// ============================================================

fn create_grant(ctx: &Ctx, n_milestones: u32) -> u64 {
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100);
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(n_milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Test Grant"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_create)
}

fn select_grant_winner(ctx: &Ctx, grant_id: u64, recipient: &Address) {
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: recipient.clone(),
            position: 1,
            credit_earn: 0, // ignored for Multi; payment-time bumps apply per milestone
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&grant_id, &winners, &op_select);
}

#[test]
fn claim_milestone_pays_per_milestone_amount() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 4);
    select_grant_winner(&ctx, grant_id, &recipient);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let recipient_before = token.balance(&recipient);

    let op_claim = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &0_u32, &3_u32, &5_u32, &op_claim);

    // Per-milestone amount: total_budget * 100% / 4 = TOTAL_BUDGET / 4
    let per_milestone = TOTAL_BUDGET / 4;
    assert_eq!(token.balance(&recipient) - recipient_before, per_milestone);

    // Profile: bootstrap (10) + earn 3 = 13 credits, reputation 5.
    let profile = ctx.profile.get_profile(&recipient).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS + 3);
    assert_eq!(profile.reputation, 5);

    // Earnings registered.
    let earnings = ctx.profile.get_earnings(&recipient, &ctx.token_addr);
    assert_eq!(earnings, per_milestone);

    // Event still Active, remaining_escrow decremented.
    let event = ctx.events.get_event(&grant_id);
    assert_eq!(event.status, EventStatus::Active);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET - per_milestone);
}

#[test]
fn claim_milestone_idempotent_per_recipient_and_milestone() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 4);
    select_grant_winner(&ctx, grant_id, &recipient);

    let op1 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &0_u32, &3_u32, &5_u32, &op1);

    // Same milestone, different op_id — should still revert.
    let op2 = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&grant_id, &recipient, &0_u32, &3_u32, &5_u32, &op2);
    assert!(res.is_err(), "same milestone twice should revert");

    // Different milestone — should succeed.
    let op3 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &1_u32, &3_u32, &5_u32, &op3);
}

#[test]
fn claim_milestone_invalid_milestone_index_reverts() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 4);
    select_grant_winner(&ctx, grant_id, &recipient);

    // Milestone index 4 is out of range for a 4-milestone grant (valid: 0..=3).
    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&grant_id, &recipient, &4_u32, &3_u32, &5_u32, &op);
    assert!(res.is_err(), "out-of-range milestone should revert");
}

#[test]
fn claim_milestone_rejects_non_grant_events() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events
        .select_winners(&bounty_id, &winners, &op_select);

    // The bounty is Completed now anyway, but claim_milestone should reject
    // even an Active Single-release event. Recreate a Single bounty without
    // winners selected:
    let bounty_id2 = create_bounty(&ctx, 0);
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_claim_milestone(
        &bounty_id2,
        &ctx.applicant,
        &0_u32,
        &3_u32,
        &5_u32,
        &op,
    );
    assert!(res.is_err(), "claim on Single-release event should revert");
}

#[test]
fn claim_milestone_final_milestone_marks_event_completed() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 2);
    select_grant_winner(&ctx, grant_id, &recipient);

    let op_a = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &0_u32, &3_u32, &5_u32, &op_a);

    let mid = ctx.events.get_event(&grant_id);
    assert_eq!(mid.status, EventStatus::Active);

    let op_b = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &1_u32, &3_u32, &5_u32, &op_b);

    let after = ctx.events.get_event(&grant_id);
    assert_eq!(after.status, EventStatus::Completed);
    assert_eq!(after.remaining_escrow, 0);
}

// ============================================================
// submit / withdraw_submission
// ============================================================

fn create_hackathon(ctx: &Ctx) -> u64 {
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Test Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_create)
}

#[test]
fn hackathon_submit_creates_anchor_without_prior_apply() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../project.json");
    let op = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op);

    let sub = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(sub.applicant, ctx.applicant);
    assert_eq!(sub.content_uri, uri);
    assert_eq!(sub.submitted_at, ctx.env.ledger().timestamp());
}

#[test]
fn bounty_submit_requires_prior_application() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../bounty.json");
    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_submit(&id, &ctx.applicant, &uri, &op);
    assert!(res.is_err(), "submit before apply on bounty should revert");
}

#[test]
fn bounty_submit_succeeds_after_apply() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &op_apply);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../bounty.json");
    let op_submit = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op_submit);

    let sub = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(sub.content_uri, uri);
}

#[test]
fn resubmit_preserves_original_submitted_at_and_updates_uri() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri_a = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op_a = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri_a, &op_a);

    let first = ctx.events.get_submission(&id, &ctx.applicant);
    let first_time = first.submitted_at;

    let uri_b = String::from_str(&ctx.env, "ipfs://Qm.../v2.json");
    let op_b = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri_b, &op_b);

    let second = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(second.content_uri, uri_b);
    assert_eq!(second.submitted_at, first_time, "submitted_at must be preserved across re-submit");
}

#[test]
fn submit_replayed_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op);

    let res = ctx
        .events
        .try_submit(&id, &ctx.applicant, &uri, &op);
    assert!(res.is_err(), "replayed submit should revert");
}

#[test]
fn withdraw_submission_removes_anchor() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op_submit = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op_submit);

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_submission(&id, &ctx.applicant, &op_wd);

    let res = ctx.events.try_get_submission(&id, &ctx.applicant);
    assert!(res.is_err(), "withdrawn submission should not be readable");
}

#[test]
fn withdraw_submission_without_submission_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let op_wd = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_withdraw_submission(&id, &ctx.applicant, &op_wd);
    assert!(res.is_err(), "withdraw without prior submission should revert");
}

// ============================================================
// Per-event fee_bps_override
//
// Sales-side discount / comp / waiver lives on the event record, not the
// global admin config. Tests below cover:
//   1. create_event with Some(override) charges the override rate.
//   2. add_funds reads the override from the event (not the live default).
//   3. waiver (Some(0)) costs the owner exactly total_budget with no fee.
//   4. an admin change to the global default does not retroactively re-price
//      in-flight events that snapshotted an override.
//   5. publish rejects override > MAX_FEE_BPS.
// ============================================================
#[test]
fn create_event_charges_override_rate_when_provided() {
    let ctx = setup();
    // Override to 1.5% (Hackathon launch rate) while the contract default is 2.5%.
    let override_bps: u32 = 150;
    let total_budget: i128 = 100_000_0000000_i128; // 100k USDC
    let expected_fee = total_budget * (override_bps as i128) / 10_000;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Hackathon at 1.5%"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(override_bps),
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op);

    // Owner paid total_budget + override_fee.
    let owner_after = token.balance(&ctx.owner);
    assert_eq!(owner_before - owner_after, total_budget + expected_fee);

    // Fee account received exactly the override fee.
    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after - fee_before, expected_fee);

    // Event record snapshotted the override.
    let event = ctx.events.get_event(&id);
    assert_eq!(event.fee_bps_override, Some(override_bps));
    assert_eq!(event.remaining_escrow, total_budget);
}

#[test]
fn add_funds_uses_event_override_not_global() {
    let ctx = setup();
    // Publish at a 0.5% promo rate.
    let override_bps: u32 = 50;
    let total_budget: i128 = 10_000_0000000_i128;

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Promo Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(override_bps),
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op_create);

    // Admin bumps global default upward; in-flight event must not re-price.
    ctx.events.set_fee_bps(&500);

    // Partner contributes 1_000 USDC. The fee should be at 0.5% (override),
    // not 5% (new global). Mint 2_000 USDC so the partner balance comfortably
    // covers amount + fee in either branch (so the test fails on the math
    // assertion if the override is ignored, not on a balance error).
    let partner = Address::generate(&ctx.env);
    let token_admin = token::StellarAssetClient::new(&ctx.env, &ctx.token_addr);
    token_admin.mint(&partner, &2_000_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let partner_before = token.balance(&partner);
    let fee_before = token.balance(&ctx.fee_account);

    let amount = 1_000_0000000_i128;
    let expected_fee = amount * (override_bps as i128) / 10_000;
    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &amount, &op_add);

    let partner_after = token.balance(&partner);
    assert_eq!(partner_before - partner_after, amount + expected_fee);
    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after - fee_before, expected_fee);
}

#[test]
fn create_event_with_waiver_charges_no_fee() {
    let ctx = setup();
    let total_budget: i128 = 5_000_0000000_i128;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Comped Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(0),
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op);

    let owner_after = token.balance(&ctx.owner);
    assert_eq!(owner_before - owner_after, total_budget, "waiver: no fee");
    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after, fee_before, "waiver: fee account unchanged");
}

#[test]
fn create_event_rejects_override_above_max_fee_bps() {
    let ctx = setup();

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: 1_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Bad rate"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        // 60% is above the MAX_FEE_BPS = 1000 cap (post L4 audit fix).
        fee_bps_override: Some(6000),
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "override > MAX_FEE_BPS must revert");
}

#[test]
fn create_event_omitted_override_falls_back_to_global_default() {
    let ctx = setup();
    let total_budget: i128 = 20_000_0000000_i128;
    let expected_fee = total_budget * (FEE_BPS as i128) / 10_000; // contract default 2.5%

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Default rate hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op);

    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after - fee_before, expected_fee);
    // Event keeps None so future add_funds also pulls the live default.
    let event = ctx.events.get_event(&id);
    assert_eq!(event.fee_bps_override, None);
}

// ============================================================
// select_winners replay lock
//
// Calling select_winners twice would silently overwrite the anchor winner
// records. The contract now rejects the second call so that downstream
// claim_milestone reads and off-chain audits read a stable winner set.
// Tested on Grant (Multi); the same code path serves Single releases.
// ============================================================
#[test]
fn select_winners_rejects_second_call_winners_already_selected() {
    let ctx = setup();
    let r1 = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 2);
    select_grant_winner(&ctx, grant_id, &r1);

    // Second call (different recipient, fresh op_id) must be rejected.
    let r2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: r2.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&grant_id, &winners, &op);
    assert!(res.is_err(), "second select_winners must revert");

    // First selection is untouched.
    let recorded = ctx.events.get_winners(&grant_id);
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded.get(0).unwrap().recipient, r1);
}

// ============================================================
// Grant last-milestone sweep
//
// Per-milestone math floors total_budget * percent / 100 / n_milestones. With
// a budget that does not divide evenly across milestones the floored amount
// strands a small residue in escrow. The last milestone for the recipient
// now sweeps that residue so total paid equals their full position share.
// ============================================================
#[test]
fn grant_last_milestone_sweeps_rounding_residue() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);

    // 100k / 3 milestones = 33,333.3333333 USDC each at 7 decimals.
    // Floored per-milestone: floor(100_000_0000000 / 3) = 33_333_3333333 stroops.
    // Residue: 100_000_0000000 - 3 * 33_333_3333333 = 1 stroop.
    let grant_id = create_grant(&ctx, 3);
    select_grant_winner(&ctx, grant_id, &recipient);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&recipient);

    // Claim m0 + m1 at floored rate.
    let floored = TOTAL_BUDGET / 3;
    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &1_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    let after_two = token.balance(&recipient);
    assert_eq!(after_two - before, floored * 2);

    // Last claim: sweep so the recipient ends up with the full position share.
    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &2_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    let after_all = token.balance(&recipient);
    assert_eq!(
        after_all - before,
        TOTAL_BUDGET,
        "last milestone must sweep residue: recipient receives full position share"
    );

    // Event drained completely and marks Completed.
    let event = ctx.events.get_event(&grant_id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

// ============================================================
// M1: select_winners pays partner top-ups too
// ============================================================

#[test]
fn select_winners_pays_against_remaining_escrow_including_top_ups() {
    // Owner deposits TOTAL_BUDGET. Partner tops up another 5_000 USDC.
    // Single winner at 100% should receive owner_budget + partner_top_up
    // (net of fees). Pre-audit this was capped at TOTAL_BUDGET and the
    // top-up would have stayed trapped until cancel.
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let partner = Address::generate(&ctx.env);
    let token_admin = token::StellarAssetClient::new(&ctx.env, &ctx.token_addr);
    let top_up: i128 = 5_000_0000000_i128;
    let top_up_fee = top_up * FEE_BPS as i128 / 10_000_i128;
    token_admin.mint(&partner, &(top_up + top_up_fee));

    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&bounty_id, &partner, &top_up, &op_add);

    // Confirm the event escrow grew.
    let event_pre = ctx.events.get_event(&bounty_id);
    assert_eq!(event_pre.remaining_escrow, TOTAL_BUDGET + top_up);

    // Single winner at 100%.
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    // Winner receives the full live escrow at select time, not the
    // original total_budget.
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET + top_up);

    // Event drained, status Completed, no orphaned partner top-up.
    let event_post = ctx.events.get_event(&bounty_id);
    assert_eq!(event_post.remaining_escrow, 0);
    assert_eq!(event_post.status, EventStatus::Completed);
}

// ============================================================
// MANAGEMENT AUTHORITY (manager decoupled from funder/owner)
// ============================================================
fn create_bounty_with_manager(ctx: &Ctx, manager: &Address) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: 10_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/events/draft/m"),
        title: String::from_str(&ctx.env, "Managed Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 1,
        fee_bps_override: None,
        manager: Some(manager.clone()),
    };
    let op_id = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_id)
}

#[test]
fn manager_defaults_to_owner_and_override_is_recorded() {
    let ctx = setup();

    // No override: management falls back to the funder/owner (legacy behavior).
    let default_id = create_bounty(&ctx, 1);
    assert_eq!(ctx.events.get_manager(&default_id), ctx.owner);

    // Override: management is the org wallet, distinct from the funder.
    let manager = Address::generate(&ctx.env);
    let managed_id = create_bounty_with_manager(&ctx, &manager);
    assert_eq!(ctx.events.get_manager(&managed_id), manager);
    assert_ne!(ctx.events.get_manager(&managed_id), ctx.owner);
}

#[test]
fn manager_can_be_rotated() {
    let ctx = setup();
    let manager = Address::generate(&ctx.env);
    let id = create_bounty_with_manager(&ctx, &manager);
    assert_eq!(ctx.events.get_manager(&id), manager);

    let manager2 = Address::generate(&ctx.env);
    ctx.events.set_manager(&id, &manager2);
    assert_eq!(ctx.events.get_manager(&id), manager2);
}

#[test]
fn manager_override_can_select_winners() {
    let ctx = setup();
    let manager = Address::generate(&ctx.env);
    let bounty_id = create_bounty_with_manager(&ctx, &manager);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Managed event settled via the manager authority.
    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
}
