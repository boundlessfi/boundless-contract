// boundless-events: bounty pillar tests (#33).
//
// Covers apply_to_bounty / withdraw_application + credit charge/refund,
// validate_create (Single release required), submission gate, select_winners.

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

fn create_bounty(ctx: &Ctx, credit_cost: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/bounty/1"),
        title: String::from_str(&ctx.env, "Test Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: credit_cost,
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn bounty_create_with_single_release_succeeds() {
    let ctx = setup();
    let id = create_bounty(&ctx, 0);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Bounty);
    assert_eq!(event.release_kind, ReleaseKind::Single);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET);
}

#[test]
fn bounty_create_with_multi_release_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/bounty"),
        title: String::from_str(&ctx.env, "Bad Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    assert!(ctx.events.try_create_event(&params, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// apply_to_bounty
// ============================================================

#[test]
fn apply_bootstraps_profile_and_charges_credits() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);

    assert!(ctx.profile.get_profile(&ctx.applicant).is_none());
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));

    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 1);

    let applicants = ctx.events.get_applicants(&id);
    assert_eq!(applicants.len(), 1);
    assert_eq!(applicants.get(0).unwrap(), ctx.applicant);
}

#[test]
fn apply_with_zero_cost_does_not_drain_credits() {
    let ctx = setup();
    let id = create_bounty(&ctx, 0);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));
    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS);
}

#[test]
fn duplicate_apply_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));
    assert!(ctx.events.try_apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn insufficient_credits_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 100);
    assert!(ctx.events.try_apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn apply_replay_same_op_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    let op = BytesN::random(&ctx.env);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &op);
    assert!(ctx.events.try_apply_to_bounty(&id, &ctx.applicant, &op).is_err());
}

// ============================================================
// withdraw_application
// ============================================================

#[test]
fn withdraw_refunds_half_credits_and_removes_applicant() {
    let ctx = setup();
    let id = create_bounty(&ctx, 2);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));

    let after_apply = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(after_apply.credits, BOOTSTRAP_CREDITS - 2);

    ctx.events.withdraw_application(&id, &ctx.applicant, &BytesN::random(&ctx.env));

    let after_wd = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(after_wd.credits, BOOTSTRAP_CREDITS - 2 + 1); // refund = cost / 2 = 1

    assert_eq!(ctx.events.get_applicants(&id).len(), 0);
}

#[test]
fn withdraw_without_prior_apply_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    assert!(ctx.events.try_withdraw_application(&id, &ctx.applicant, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn withdraw_after_submission_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));

    let uri = String::from_str(&ctx.env, "ipfs://Qm/bounty.json");
    ctx.events.submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env));

    assert!(ctx.events.try_withdraw_application(&id, &ctx.applicant, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// Submission gate
// ============================================================

#[test]
fn submit_without_prior_apply_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    let uri = String::from_str(&ctx.env, "ipfs://Qm/bounty.json");
    assert!(ctx.events.try_submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn submit_after_apply_succeeds() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));
    let uri = String::from_str(&ctx.env, "ipfs://Qm/bounty.json");
    ctx.events.submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env));
    let sub = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(sub.content_uri, uri);
}

// ============================================================
// select_winners
// ============================================================

#[test]
fn select_winners_pays_recipient_and_updates_profile() {
    let ctx = setup();
    let id = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(&id, &ctx.applicant, &BytesN::random(&ctx.env));

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
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 1 + 20);
    assert_eq!(profile.reputation, 50);
    assert_eq!(ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr), TOTAL_BUDGET);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn select_winners_rejects_invalid_position() {
    let ctx = setup();
    let id = create_bounty(&ctx, 0);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: ctx.applicant.clone(), position: 2, credit_earn: 0, reputation_bump: 0 },
    ];
    assert!(ctx.events.try_select_winners(&id, &winners, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn select_winners_twice_reverts() {
    let ctx = setup();
    let id = create_bounty(&ctx, 0);
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
fn cancel_bounty_refunds_owner() {
    let ctx = setup();
    let id = create_bounty(&ctx, 0);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&ctx.owner);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(token.balance(&ctx.owner) - before, TOTAL_BUDGET);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelled);
}
