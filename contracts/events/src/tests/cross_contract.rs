#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

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
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id = env.register(ProfileContract, (profile_admin.clone(),));
    let profile = ProfileContractClient::new(&env, &profile_id);

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

    profile.set_events_contract(&events_id);

    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let token_addr = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_addr);

    token_admin.mint(&fee_account, &0);

    let owner = Address::generate(&env);
    token_admin.mint(&owner, &1_000_000_0000000_i128);

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

fn create_bounty(ctx: &Ctx) -> u64 {
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
    let bounty_id = create_bounty(&ctx);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let claim_op = BytesN::random(&ctx.env);
    ctx.events.claim_prize(&bounty_id, &ctx.applicant, &1_u32, &50_u32, &claim_op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account), FEE_AMOUNT);

    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.reputation, 50);

    let earnings = ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr);
    assert_eq!(earnings, TOTAL_BUDGET);

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);

    let winner_list = ctx.events.get_winners(&bounty_id);
    assert_eq!(winner_list.len(), 2);
    let anchor = winner_list.get(0).unwrap();
    assert_eq!(anchor.recipient, ctx.applicant);
    assert_eq!(anchor.position, 1);
    assert_eq!(anchor.amount, TOTAL_BUDGET);
    assert_eq!(anchor.milestone, None);
    assert!(anchor.paid_at.is_none());
    let claim = winner_list.get(1).unwrap();
    assert_eq!(claim.recipient, ctx.applicant);
    assert_eq!(claim.position, 1);
    assert_eq!(claim.amount, TOTAL_BUDGET);
    assert_eq!(claim.milestone, None);
    assert!(claim.paid_at.is_some());
}

#[test]
fn select_winners_requires_position_in_distribution() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 2, // distribution only has position 1
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
            reputation_bump: 50,
        },
        WinnerSpec {
            recipient: other_recipient,
            position: 1, // duplicate
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
            reputation_bump: 50,
        },
        WinnerSpec {
            recipient: winner_b.clone(),
            position: 2,
            reputation_bump: 25,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Pull-model: each winner claims their prize.
    ctx.events.claim_prize(&bounty_id, &winner_a, &1_u32, &50_u32, &BytesN::random(&ctx.env));
    ctx.events.claim_prize(&bounty_id, &winner_b, &2_u32, &25_u32, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let amount_a = TOTAL_BUDGET * 60 / 100;
    let amount_b = TOTAL_BUDGET * 40 / 100;
    assert_eq!(token.balance(&winner_a), amount_a);
    assert_eq!(token.balance(&winner_b), amount_b);

    let profile_a = ctx.profile.get_profile(&winner_a).unwrap();
    let profile_b = ctx.profile.get_profile(&winner_b).unwrap();
    assert_eq!(profile_a.reputation, 50);
    assert_eq!(profile_b.reputation, 25);

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn select_winners_replayed_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
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
    let bounty_id = create_bounty(&ctx);

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
    let bounty_id = create_bounty(&ctx);

    drive_cancel(&ctx.env, &ctx.events, bounty_id);

    let op_again = BytesN::random(&ctx.env);
    let res = ctx.events.try_start_cancel(&bounty_id, &op_again);
    assert!(res.is_err(), "second cancel should revert");
}

#[test]
fn cancel_after_select_winners_refunds_only_remaining() {
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
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/partial"),
        title: String::from_str(&ctx.env, "Partial Pay Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let bounty_id = ctx.events.create_event(&params, &op_create);

    let winner_a = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: winner_a.clone(),
            position: 1,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let claim_op = BytesN::random(&ctx.env);
    ctx.events.claim_prize(&bounty_id, &winner_a, &1_u32, &50_u32, &claim_op);

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
        .claim_milestone(&grant_id, &recipient, &0_u32, &5_u32, &op_claim);

    let per_milestone = TOTAL_BUDGET / 4;
    assert_eq!(token.balance(&recipient) - recipient_before, per_milestone);

    let profile = ctx.profile.get_profile(&recipient).unwrap();
    assert_eq!(profile.reputation, 5);

    let earnings = ctx.profile.get_earnings(&recipient, &ctx.token_addr);
    assert_eq!(earnings, per_milestone);

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
        .claim_milestone(&grant_id, &recipient, &0_u32, &5_u32, &op1);

    let op2 = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&grant_id, &recipient, &0_u32, &5_u32, &op2);
    assert!(res.is_err(), "same milestone twice should revert");

    let op3 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &1_u32, &5_u32, &op3);
}

#[test]
fn claim_milestone_invalid_milestone_index_reverts() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 4);
    select_grant_winner(&ctx, grant_id, &recipient);

    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&grant_id, &recipient, &4_u32, &5_u32, &op);
    assert!(res.is_err(), "out-of-range milestone should revert");
}

#[test]
fn claim_milestone_rejects_non_grant_events() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    let bounty_id2 = create_bounty(&ctx);
    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&bounty_id2, &ctx.applicant, &0_u32, &5_u32, &op);
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
        .claim_milestone(&grant_id, &recipient, &0_u32, &5_u32, &op_a);

    let mid = ctx.events.get_event(&grant_id);
    assert_eq!(mid.status, EventStatus::Active);

    let op_b = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&grant_id, &recipient, &1_u32, &5_u32, &op_b);

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
    let id = create_bounty(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../bounty.json");
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_submit(&id, &ctx.applicant, &uri, &op);
    assert!(res.is_err(), "submit before apply on bounty should revert");
}

#[test]
fn bounty_submit_succeeds_after_apply() {
    let ctx = setup();
    let id = create_bounty(&ctx);

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
    assert_eq!(
        second.submitted_at, first_time,
        "submitted_at must be preserved across re-submit"
    );
}

#[test]
fn submit_replayed_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op);

    let res = ctx.events.try_submit(&id, &ctx.applicant, &uri, &op);
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
    ctx.events.withdraw_submission(&id, &ctx.applicant, &op_wd);

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
    assert!(
        res.is_err(),
        "withdraw without prior submission should revert"
    );
}

// ============================================================
// Per-event fee_bps_override
// ============================================================
#[test]
fn create_event_charges_override_rate_when_provided() {
    let ctx = setup();
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
        fee_bps_override: Some(override_bps),
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op);

    let owner_after = token.balance(&ctx.owner);
    assert_eq!(owner_before - owner_after, total_budget + expected_fee);

    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after - fee_before, expected_fee);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.fee_bps_override, Some(override_bps));
    assert_eq!(event.remaining_escrow, total_budget);
}

#[test]
fn add_funds_uses_event_override_not_global() {
    let ctx = setup();
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
        fee_bps_override: Some(override_bps),
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op_create);

    ctx.events.set_fee_bps(&500);

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
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op);

    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(fee_after - fee_before, expected_fee);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.fee_bps_override, None);
}

// ============================================================
// select_winners replay lock
// ============================================================
#[test]
fn select_winners_rejects_second_call_winners_already_selected() {
    let ctx = setup();
    let r1 = Address::generate(&ctx.env);
    let grant_id = create_grant(&ctx, 2);
    select_grant_winner(&ctx, grant_id, &r1);

    let r2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: r2.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&grant_id, &winners, &op);
    assert!(res.is_err(), "second select_winners must revert");

    let recorded = ctx.events.get_winners(&grant_id);
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded.get(0).unwrap().recipient, r1);
}

// ============================================================
// Grant last-milestone sweep
// ============================================================
#[test]
fn grant_last_milestone_sweeps_rounding_residue() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);

    let grant_id = create_grant(&ctx, 3);
    select_grant_winner(&ctx, grant_id, &recipient);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&recipient);

    let floored = TOTAL_BUDGET / 3;
    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &1_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    let after_two = token.balance(&recipient);
    assert_eq!(after_two - before, floored * 2);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &2_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    let after_all = token.balance(&recipient);
    assert_eq!(
        after_all - before,
        TOTAL_BUDGET,
        "last milestone must sweep residue: recipient receives full position share"
    );

    let event = ctx.events.get_event(&grant_id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

// ============================================================
// M1: select_winners pays partner top-ups too
// ============================================================

#[test]
fn select_winners_pays_against_remaining_escrow_including_top_ups() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let partner = Address::generate(&ctx.env);
    let token_admin = token::StellarAssetClient::new(&ctx.env, &ctx.token_addr);
    let top_up: i128 = 5_000_0000000_i128;
    let top_up_fee = top_up * FEE_BPS as i128 / 10_000_i128;
    token_admin.mint(&partner, &(top_up + top_up_fee));

    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&bounty_id, &partner, &top_up, &op_add);

    let event_pre = ctx.events.get_event(&bounty_id);
    assert_eq!(event_pre.remaining_escrow, TOTAL_BUDGET + top_up);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Pull-model: claim prize.
    ctx.events.claim_prize(&bounty_id, &ctx.applicant, &1_u32, &50_u32, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET + top_up);

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
        fee_bps_override: None,
        manager: Some(manager.clone()),
    };
    let op_id = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_id)
}

#[test]
fn manager_defaults_to_owner_and_override_is_recorded() {
    let ctx = setup();

    let default_id = create_bounty(&ctx);
    assert_eq!(ctx.events.get_manager(&default_id), ctx.owner);

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
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    ctx.events.claim_prize(&bounty_id, &ctx.applicant, &1_u32, &0_u32, &BytesN::random(&ctx.env));

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);
}
