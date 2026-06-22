// boundless-events: escrow fee and release math tests.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::errors::Error;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};
use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250;
const TOTAL_BUDGET: i128 = 1_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    #[allow(dead_code)]
    profile: ProfileContractClient<'a>,
    owner: Address,
    token_addr: Address,
    fee_account: Address,
    token_admin: token::StellarAssetClient<'a>,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id = env.register(ProfileContract, (profile_admin, BOOTSTRAP_CREDITS));
    let profile = ProfileContractClient::new(&env, &profile_id);

    let events_admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (
            events_admin,
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
    token_admin.mint(&owner, &10_000_0000000_i128);
    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        profile,
        owner,
        token_addr,
        fee_account,
        token_admin,
    }
}

fn distribution(env: &Env, a: u32, b: Option<u32>) -> Map<u32, u32> {
    let mut dist = Map::new(env);
    dist.set(1, a);
    if let Some(second) = b {
        dist.set(2, second);
    }
    dist
}

fn create_hackathon(ctx: &Ctx, total_budget: i128, fee_bps_override: Option<u32>) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/escrow-math"),
        title: String::from_str(&ctx.env, "Escrow Math"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: distribution(&ctx.env, 100, None),
        application_credit_cost: 0,
        fee_bps_override,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

fn create_grant(ctx: &Ctx, total_budget: i128, milestones: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/escrow-grant"),
        title: String::from_str(&ctx.env, "Escrow Grant"),
        deadline: None,
        winner_distribution: distribution(&ctx.env, 100, None),
        application_credit_cost: 0,
        fee_bps_override: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

#[test]
fn create_event_default_fee_rounds_down_to_stroops() {
    let ctx = setup();
    let amount = 1_000_0000001_i128;
    let expected_fee = amount * FEE_BPS as i128 / 10_000_i128;
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    let id = create_hackathon(&ctx, amount, None);

    assert_eq!(token.balance(&ctx.owner), owner_before - amount - expected_fee);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, amount);
    assert_eq!(event.fee_bps_override, None);
}

#[test]
fn create_event_override_fee_is_charged_and_stored() {
    let ctx = setup();
    let amount = 2_000_0000001_i128;
    let override_bps = 75_u32;
    let expected_fee = amount * override_bps as i128 / 10_000_i128;
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    let id = create_hackathon(&ctx, amount, Some(override_bps));

    assert_eq!(token.balance(&ctx.owner), owner_before - amount - expected_fee);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.fee_bps_override, Some(override_bps));
}

#[test]
fn create_event_with_zero_override_charges_no_fee() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    create_hackathon(&ctx, TOTAL_BUDGET, Some(0));

    assert_eq!(token.balance(&ctx.owner), owner_before - TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account), fee_before);
}

#[test]
fn create_event_rejects_fee_override_above_cap() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/bad-fee"),
        title: String::from_str(&ctx.env, "Bad Fee"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: distribution(&ctx.env, 100, None),
        application_credit_cost: 0,
        fee_bps_override: Some(1_001),
    };

    let err = ctx
        .events
        .try_create_event(&params, &BytesN::random(&ctx.env))
        .err()
        .expect("fee override above cap should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidFeeBps);
}

#[test]
fn add_funds_uses_event_override_after_global_fee_changes() {
    let ctx = setup();
    let override_bps = 50_u32;
    let id = create_hackathon(&ctx, TOTAL_BUDGET, Some(override_bps));
    ctx.events.set_fee_bps(&900);

    let partner = Address::generate(&ctx.env);
    let amount = 500_0000000_i128;
    ctx.token_admin.mint(&partner, &1_000_0000000_i128);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let partner_before = token.balance(&partner);
    let fee_before = token.balance(&ctx.fee_account);
    let expected_fee = amount * override_bps as i128 / 10_000_i128;

    ctx.events
        .add_funds(&id, &partner, &amount, &BytesN::random(&ctx.env));

    assert_eq!(partner_before - token.balance(&partner), amount + expected_fee);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, TOTAL_BUDGET + amount);
}

#[test]
fn add_funds_without_override_uses_current_global_fee() {
    let ctx = setup();
    let id = create_hackathon(&ctx, TOTAL_BUDGET, None);
    let new_bps = 900_u32;
    ctx.events.set_fee_bps(&new_bps);

    let partner = Address::generate(&ctx.env);
    let amount = 500_0000000_i128;
    ctx.token_admin.mint(&partner, &1_000_0000000_i128);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);
    let expected_fee = amount * new_bps as i128 / 10_000_i128;

    ctx.events
        .add_funds(&id, &partner, &amount, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&ctx.fee_account) - fee_before, expected_fee);
}

#[test]
fn select_winners_uses_live_escrow_and_leaves_flooring_dust() {
    let ctx = setup();
    let total_budget = 1_000_0000001_i128;
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/flooring"),
        title: String::from_str(&ctx.env, "Flooring"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: distribution(&ctx.env, 33, Some(67)),
        application_credit_cost: 0,
        fee_bps_override: Some(0),
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));
    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: w1.clone(),
            position: 1,
            credit_earn: 1,
            reputation_bump: 1,
        },
        WinnerSpec {
            recipient: w2.clone(),
            position: 2,
            credit_earn: 1,
            reputation_bump: 1,
        },
    ];

    ctx.events
        .select_winners(&id, &winners, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&w1), total_budget * 33 / 100);
    assert_eq!(token.balance(&w2), total_budget * 67 / 100);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 1);
    assert_eq!(event.status, EventStatus::Active);
}

#[test]
fn grant_last_milestone_sweeps_release_rounding_residue() {
    let ctx = setup();
    let total_budget = 1_000_0000001_i128;
    let id = create_grant(&ctx, total_budget, 3);
    let recipient = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: recipient.clone(),
            position: 1,
            credit_earn: 1,
            reputation_bump: 1,
        },
    ];
    ctx.events
        .select_winners(&id, &winners, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&recipient);
    let floored = total_budget / 3;
    ctx.events.claim_milestone(
        &id,
        &recipient,
        &0_u32,
        &1_u32,
        &1_u32,
        &BytesN::random(&ctx.env),
    );
    ctx.events.claim_milestone(
        &id,
        &recipient,
        &1_u32,
        &1_u32,
        &1_u32,
        &BytesN::random(&ctx.env),
    );
    assert_eq!(token.balance(&recipient) - before, floored * 2);

    ctx.events.claim_milestone(
        &id,
        &recipient,
        &2_u32,
        &1_u32,
        &1_u32,
        &BytesN::random(&ctx.env),
    );

    assert_eq!(token.balance(&recipient) - before, total_budget);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}
