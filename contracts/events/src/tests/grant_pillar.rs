// boundless-events: grant pillar tests (#32).
//
// Covers validate_create (Multi release required) + claim_milestone
// fixed-split math, last-milestone dust sweep, credit/rep side-effects,
// and error variants.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

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

    Ctx { env, events, profile, owner, token_addr, token_admin, fee_account }
}

fn single_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_grant(ctx: &Ctx, n: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(n),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant/1"),
        title: String::from_str(&ctx.env, "Test Grant"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

fn select_winner(ctx: &Ctx, id: u64, recipient: &Address) {
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: recipient.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn grant_create_with_multi_release_succeeds() {
    let ctx = setup();
    let id = create_grant(&ctx, 3);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Grant);
    assert_eq!(event.release_kind, ReleaseKind::Multi(3));
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET);
}

#[test]
fn grant_create_with_single_release_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Bad Grant"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    assert!(ctx.events.try_create_event(&params, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn grant_create_with_zero_milestones_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(0),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Zero Milestones"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    assert!(ctx.events.try_create_event(&params, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// claim_milestone: fixed-split math
// ============================================================

#[test]
fn claim_milestone_pays_fixed_per_milestone_amount() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 4);
    select_winner(&ctx, id, &recipient);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&recipient);
    let fee_before = token.balance(&ctx.fee_account);

    ctx.events.claim_milestone(&id, &recipient, &0_u32, &3_u32, &5_u32, &BytesN::random(&ctx.env));

    let per_milestone = TOTAL_BUDGET / 4;
    assert_eq!(token.balance(&recipient) - before, per_milestone);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, TOTAL_BUDGET - per_milestone);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);
}

#[test]
fn claim_milestone_last_sweeps_rounding_residue() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 3);
    select_winner(&ctx, id, &recipient);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&recipient);
    let fee_before = token.balance(&ctx.fee_account);
    let floored = TOTAL_BUDGET / 3;

    ctx.events.claim_milestone(&id, &recipient, &0_u32, &0, &0, &BytesN::random(&ctx.env));
    ctx.events.claim_milestone(&id, &recipient, &1_u32, &0, &0, &BytesN::random(&ctx.env));
    assert_eq!(token.balance(&recipient) - before, floored * 2);

    ctx.events.claim_milestone(&id, &recipient, &2_u32, &0, &0, &BytesN::random(&ctx.env));
    assert_eq!(token.balance(&recipient) - before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

#[test]
fn claim_milestone_marks_completed_on_last() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 2);
    select_winner(&ctx, id, &recipient);

    ctx.events.claim_milestone(&id, &recipient, &0_u32, &0, &0, &BytesN::random(&ctx.env));
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);

    ctx.events.claim_milestone(&id, &recipient, &1_u32, &0, &0, &BytesN::random(&ctx.env));
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);
}

// ============================================================
// claim_milestone: credit/reputation side-effects
// ============================================================

#[test]
fn claim_milestone_earns_credits_and_bumps_reputation() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 4);
    select_winner(&ctx, id, &recipient);

    ctx.events.claim_milestone(&id, &recipient, &0_u32, &5_u32, &10_u32, &BytesN::random(&ctx.env));

    let profile = ctx.profile.get_profile(&recipient).unwrap();
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS + 5);
    assert_eq!(profile.reputation, 10);

    let per_milestone = TOTAL_BUDGET / 4;
    assert_eq!(ctx.profile.get_earnings(&recipient, &ctx.token_addr), per_milestone);
}

// ============================================================
// Error variants
// ============================================================

#[test]
fn claim_milestone_already_claimed_reverts() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 4);
    select_winner(&ctx, id, &recipient);

    ctx.events.claim_milestone(&id, &recipient, &0_u32, &0, &0, &BytesN::random(&ctx.env));
    assert!(ctx.events.try_claim_milestone(&id, &recipient, &0_u32, &0, &0, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn claim_milestone_out_of_range_reverts() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 3);
    select_winner(&ctx, id, &recipient);
    assert!(ctx.events.try_claim_milestone(&id, &recipient, &3_u32, &0, &0, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn claim_milestone_without_being_winner_reverts() {
    let ctx = setup();
    let id = create_grant(&ctx, 3);
    let non_winner = Address::generate(&ctx.env);
    assert!(ctx.events.try_claim_milestone(&id, &non_winner, &0_u32, &0, &0, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn claim_milestone_on_single_release_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hack"),
        title: String::from_str(&ctx.env, "Single Release"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));
    let r = Address::generate(&ctx.env);
    assert!(ctx.events.try_claim_milestone(&id, &r, &0_u32, &0, &0, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn claim_milestone_op_replay_reverts() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);
    let id = create_grant(&ctx, 4);
    select_winner(&ctx, id, &recipient);

    let op = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &recipient, &0_u32, &0, &0, &op);
    assert!(ctx.events.try_claim_milestone(&id, &recipient, &0_u32, &0, &0, &op).is_err());
}

// ============================================================
// Two-winner grant
// ============================================================

#[test]
fn two_winner_grant_each_claims_their_share() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(2),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/multi-grant"),
        title: String::from_str(&ctx.env, "Multi Winner Grant"),
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
        WinnerSpec { recipient: w2.clone(), position: 2, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let w1_before = token.balance(&w1);
    let w2_before = token.balance(&w2);
    let fee_before = token.balance(&ctx.fee_account);

    ctx.events.claim_milestone(&id, &w1, &0_u32, &0, &0, &BytesN::random(&ctx.env));
    ctx.events.claim_milestone(&id, &w1, &1_u32, &0, &0, &BytesN::random(&ctx.env));
    ctx.events.claim_milestone(&id, &w2, &0_u32, &0, &0, &BytesN::random(&ctx.env));
    ctx.events.claim_milestone(&id, &w2, &1_u32, &0, &0, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&w1) - w1_before, TOTAL_BUDGET * 60 / 100);
    assert_eq!(token.balance(&w2) - w2_before, TOTAL_BUDGET * 40 / 100);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 0);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);
}
