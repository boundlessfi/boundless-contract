// boundless-events: hackathon pillar tests (#34).
//
// Covers validate_create (Single release + deadline required),
// open submission model (no apply step), select_winners distribution,
// and cancel refund.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};
use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250;
const TOTAL_BUDGET: i128 = 10_000_0000000_i128;

#[allow(dead_code)]
struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    profile: ProfileContractClient<'a>,
    owner: Address,
    applicant: Address,
    token_addr: Address,
    token_admin: token::StellarAssetClient<'a>,
    fee_account: Address,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id = env.register(ProfileContract, (profile_admin.clone(), BOOTSTRAP_CREDITS));
    let profile = ProfileContractClient::new(&env, &profile_id);

    let events_admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (events_admin.clone(), fee_account.clone(), FEE_BPS, profile_id.clone()),
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

    Ctx { env, events, profile, owner, applicant, token_addr, token_admin, fee_account }
}

fn single_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_hackathon(ctx: &Ctx) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon/1"),
        title: String::from_str(&ctx.env, "Test Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn hackathon_create_succeeds_with_single_release_and_deadline() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Hackathon);
    assert_eq!(event.release_kind, ReleaseKind::Single);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET);
    assert!(event.deadline.is_some());
}

#[test]
fn hackathon_create_without_deadline_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "No Deadline"),
        deadline: None,
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    assert!(ctx.events.try_create_event(&params, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn hackathon_create_with_multi_release_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Multi Release Hack"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    assert!(ctx.events.try_create_event(&params, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// Open submission model (no apply step needed)
// ============================================================

#[test]
fn submit_without_prior_apply_succeeds() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let uri = String::from_str(&ctx.env, "ipfs://Qm/project.json");
    ctx.events.submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env));
    let sub = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(sub.content_uri, uri);
}

#[test]
fn resubmit_updates_uri_preserves_submitted_at() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri_a = String::from_str(&ctx.env, "ipfs://Qm/v1.json");
    ctx.events.submit(&id, &ctx.applicant, &uri_a, &BytesN::random(&ctx.env));
    let first = ctx.events.get_submission(&id, &ctx.applicant);

    let uri_b = String::from_str(&ctx.env, "ipfs://Qm/v2.json");
    ctx.events.submit(&id, &ctx.applicant, &uri_b, &BytesN::random(&ctx.env));
    let second = ctx.events.get_submission(&id, &ctx.applicant);

    assert_eq!(second.content_uri, uri_b);
    assert_eq!(second.submitted_at, first.submitted_at);
}

#[test]
fn withdraw_submission_removes_entry() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let uri = String::from_str(&ctx.env, "ipfs://Qm/project.json");
    ctx.events.submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env));
    ctx.events.withdraw_submission(&id, &ctx.applicant, &BytesN::random(&ctx.env));
    assert!(ctx.events.try_get_submission(&id, &ctx.applicant).is_err());
}

#[test]
fn withdraw_without_submission_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    assert!(ctx.events.try_withdraw_submission(&id, &ctx.applicant, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// select_winners: distribution + profile
// ============================================================

#[test]
fn select_winners_single_pays_full_budget_and_updates_profile() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: ctx.applicant.clone(), position: 1, credit_earn: 20, reputation_bump: 50 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET);
    let expected_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000;
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);

    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS + 20);
    assert_eq!(profile.reputation, 50);
    assert_eq!(ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr), TOTAL_BUDGET);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn select_winners_multi_recipient_distribution() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Multi Winner Hack"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 20, reputation_bump: 50 },
        WinnerSpec { recipient: w2.clone(), position: 2, credit_earn: 10, reputation_bump: 25 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&w1), TOTAL_BUDGET * 60 / 100);
    assert_eq!(token.balance(&w2), TOTAL_BUDGET * 40 / 100);
    let expected_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000;
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);

    let p1 = ctx.profile.get_profile(&w1).unwrap();
    let p2 = ctx.profile.get_profile(&w2).unwrap();
    assert_eq!(p1.credits, BOOTSTRAP_CREDITS + 20);
    assert_eq!(p1.reputation, 50);
    assert_eq!(p2.credits, BOOTSTRAP_CREDITS + 10);
    assert_eq!(p2.reputation, 25);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Completed);
}

#[test]
fn select_winners_duplicate_position_reverts() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Dup Pos Hack"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
        WinnerSpec { recipient: w2.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    assert!(ctx.events.try_select_winners(&id, &winners, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn select_winners_twice_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: ctx.applicant.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners.clone(), &BytesN::random(&ctx.env));
    assert!(ctx.events.try_select_winners(&id, &winners, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// cancel
// ============================================================

#[test]
fn cancel_hackathon_refunds_owner_in_full() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&ctx.owner);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(token.balance(&ctx.owner) - before, TOTAL_BUDGET);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelled);
}
