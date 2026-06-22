// boundless-events: grant pillar milestone tests.

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
    profile: ProfileContractClient<'a>,
    owner: Address,
    token_addr: Address,
    #[allow(dead_code)]
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
        (events_admin, fee_account.clone(), FEE_BPS, profile_id.clone()),
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
        token_admin,
    }
}

fn dist(env: &Env, percent: u32) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, percent);
    m
}

fn grant_params(ctx: &Ctx, milestones: u32) -> CreateEventParams {
    CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Grant"),
        deadline: None,
        winner_distribution: dist(&ctx.env, 100),
        application_credit_cost: 0,
        fee_bps_override: None,
    }
}

fn create_grant(ctx: &Ctx, milestones: u32) -> u64 {
    ctx.events
        .create_event(&grant_params(ctx, milestones), &BytesN::random(&ctx.env))
}

fn select_winner(ctx: &Ctx, grant_id: u64, recipient: &Address) {
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: recipient.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    ctx.events
        .select_winners(&grant_id, &winners, &BytesN::random(&ctx.env));
}

#[test]
fn grant_create_requires_multi_release_kind() {
    let ctx = setup();
    let mut params = grant_params(&ctx, 3);
    params.release_kind = ReleaseKind::Single;

    let err = ctx
        .events
        .try_create_event(&params, &BytesN::random(&ctx.env))
        .err()
        .expect("single release grant should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidReleaseKind);
}

#[test]
fn grant_create_rejects_zero_milestones() {
    let ctx = setup();

    let err = ctx
        .events
        .try_create_event(&grant_params(&ctx, 0), &BytesN::random(&ctx.env))
        .err()
        .expect("zero milestone grant should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidReleaseKind);
}

#[test]
fn select_winners_records_anchor_without_payout() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 4);
    let recipient = Address::generate(&ctx.env);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    select_winner(&ctx, grant_id, &recipient);

    assert_eq!(token.balance(&recipient), 0);
    let winners = ctx.events.get_winners(&grant_id);
    assert_eq!(winners.len(), 1);
    let anchor = winners.get(0).unwrap();
    assert_eq!(anchor.recipient, recipient);
    assert_eq!(anchor.position, 1);
    assert_eq!(anchor.amount, 0);
    assert_eq!(anchor.milestone, None);
    assert_eq!(ctx.events.get_event(&grant_id).remaining_escrow, TOTAL_BUDGET);
}

#[test]
fn claim_milestone_pays_fixed_share_and_updates_profile() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 4);
    let recipient = Address::generate(&ctx.env);
    select_winner(&ctx, grant_id, &recipient);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );

    let expected = TOTAL_BUDGET / 4;
    assert_eq!(token.balance(&recipient), expected);
    let event = ctx.events.get_event(&grant_id);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET - expected);
    assert_eq!(event.status, EventStatus::Active);

    let profile = ctx.profile.get_profile(&recipient).expect("profile exists");
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS + 3);
    assert_eq!(profile.reputation, 5);
    assert_eq!(ctx.profile.get_earnings(&recipient, &ctx.token_addr), expected);
}

#[test]
fn claim_milestone_rejects_replayed_recipient_milestone() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 4);
    let recipient = Address::generate(&ctx.env);
    select_winner(&ctx, grant_id, &recipient);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    let err = ctx
        .events
        .try_claim_milestone(
            &grant_id,
            &recipient,
            &0_u32,
            &3_u32,
            &5_u32,
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("same recipient milestone should fail")
        .unwrap();
    assert_eq!(err, Error::MilestoneAlreadyClaimed);
}

#[test]
fn claim_milestone_rejects_out_of_range_milestone() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 2);
    let recipient = Address::generate(&ctx.env);
    select_winner(&ctx, grant_id, &recipient);

    let err = ctx
        .events
        .try_claim_milestone(
            &grant_id,
            &recipient,
            &2_u32,
            &3_u32,
            &5_u32,
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("milestone index >= count should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidMilestone);
}

#[test]
fn claim_milestone_rejects_recipient_without_anchor_winner() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 2);
    let recipient = Address::generate(&ctx.env);

    let err = ctx
        .events
        .try_claim_milestone(
            &grant_id,
            &recipient,
            &0_u32,
            &3_u32,
            &5_u32,
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("recipient without winner anchor should fail")
        .unwrap();
    assert_eq!(err, Error::NoSubmissions);
}

#[test]
fn final_milestone_marks_grant_completed() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 2);
    let recipient = Address::generate(&ctx.env);
    select_winner(&ctx, grant_id, &recipient);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );
    assert_eq!(ctx.events.get_event(&grant_id).status, EventStatus::Active);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &1_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );

    let event = ctx.events.get_event(&grant_id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

#[test]
fn claim_milestone_requires_owner_auth() {
    let ctx = setup();
    let grant_id = create_grant(&ctx, 2);
    let recipient = Address::generate(&ctx.env);
    select_winner(&ctx, grant_id, &recipient);

    ctx.events.claim_milestone(
        &grant_id,
        &recipient,
        &0_u32,
        &3_u32,
        &5_u32,
        &BytesN::random(&ctx.env),
    );

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == ctx.owner),
        "grant milestone claim must demand owner auth"
    );
}
