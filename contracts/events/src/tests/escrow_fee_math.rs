// boundless-events: escrow fee + release math tests (#31).
//
// Covers default fee, fee_bps_override, waiver, override > MAX rejected,
// add_funds uses event-snapshotted rate, and release percent math.

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

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    #[allow(dead_code)]
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
    token_admin.mint(&owner, &100_000_0000000_i128);
    events.register_supported_token(&token_addr);

    Ctx { env, events, profile, owner, token_addr, token_admin, fee_account }
}

fn single_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_hackathon(ctx: &Ctx, budget: i128, override_bps: Option<u32>) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/fee-test"),
        title: String::from_str(&ctx.env, "Fee Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: override_bps,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

// ============================================================
// Default fee
// ============================================================

#[test]
fn create_event_charges_default_fee() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;
    let expected_fee = budget * FEE_BPS as i128 / 10_000;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    let id = create_hackathon(&ctx, budget, None);

    assert_eq!(owner_before - token.balance(&ctx.owner), budget + expected_fee);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, budget);
    assert_eq!(ctx.events.get_event(&id).fee_bps_override, None);
}

// ============================================================
// Override fee
// ============================================================

#[test]
fn create_event_charges_override_fee() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;
    let override_bps: u32 = 150;
    let expected_fee = budget * override_bps as i128 / 10_000;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let id = create_hackathon(&ctx, budget, Some(override_bps));

    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
    assert_eq!(ctx.events.get_event(&id).fee_bps_override, Some(override_bps));
}

#[test]
fn add_funds_uses_event_override_not_global_default() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;
    let override_bps: u32 = 50;
    let id = create_hackathon(&ctx, budget, Some(override_bps));

    ctx.events.set_fee_bps(&500);

    let partner = Address::generate(&ctx.env);
    let amount = 1_000_0000000_i128;
    let expected_fee = amount * override_bps as i128 / 10_000;
    ctx.token_admin.mint(&partner, &(amount + expected_fee + 1_000_0000000));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let partner_before = token.balance(&partner);
    let fee_before = token.balance(&ctx.fee_account);

    ctx.events.add_funds(&id, &partner, &amount, &BytesN::random(&ctx.env));

    assert_eq!(partner_before - token.balance(&partner), amount + expected_fee);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
}

// ============================================================
// Waiver (override = 0)
// ============================================================

#[test]
fn create_event_with_waiver_charges_no_fee() {
    let ctx = setup();
    let budget = 5_000_0000000_i128;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    create_hackathon(&ctx, budget, Some(0));

    assert_eq!(owner_before - token.balance(&ctx.owner), budget);
    assert_eq!(token.balance(&ctx.fee_account), fee_before);
}

// ============================================================
// Override > MAX_FEE_BPS rejected
// ============================================================

#[test]
fn create_event_rejects_override_above_max_fee_bps() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: 1_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/fee-test"),
        title: String::from_str(&ctx.env, "Bad Rate"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(6000),
        manager: None,
    };
    let res = ctx.events.try_create_event(&params, &BytesN::random(&ctx.env));
    assert!(res.is_err());
}

// ============================================================
// Release math: single winner 100%
// ============================================================

#[test]
fn select_winners_single_100_percent_drains_escrow() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;
    let id = create_hackathon(&ctx, budget, None);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);
    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&winner), budget);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

// ============================================================
// Release math: 60/40 split
// ============================================================

#[test]
fn select_winners_60_40_split_math_correct() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/split"),
        title: String::from_str(&ctx.env, "60/40 Split"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);
    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
        WinnerSpec { recipient: w2.clone(), position: 2, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&w1), budget * 60 / 100);
    assert_eq!(token.balance(&w2), budget * 40 / 100);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 0);
}

// ============================================================
// Release includes partner top-ups (M1)
// ============================================================

#[test]
fn select_winners_pays_against_full_escrow_including_top_ups() {
    let ctx = setup();
    let budget = 10_000_0000000_i128;
    let id = create_hackathon(&ctx, budget, None);

    let partner = Address::generate(&ctx.env);
    let top_up = 5_000_0000000_i128;
    let fee = top_up * FEE_BPS as i128 / 10_000;
    ctx.token_admin.mint(&partner, &(top_up + fee));
    ctx.events.add_funds(&id, &partner, &top_up, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);
    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&winner), budget + top_up);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 0);
}

// ============================================================
// Admin can update global fee_bps
// ============================================================

#[test]
fn admin_update_fee_bps_applies_to_new_events() {
    let ctx = setup();
    ctx.events.set_fee_bps(&100);

    let budget = 10_000_0000000_i128;
    let expected_fee = budget * 100 / 10_000;

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    create_hackathon(&ctx, budget, None);

    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
}
