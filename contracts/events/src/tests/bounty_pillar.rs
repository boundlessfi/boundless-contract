// boundless-events: bounty pillar apply / withdraw tests.

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
const TOTAL_BUDGET: i128 = 10_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    profile: ProfileContractClient<'a>,
    owner: Address,
    applicant: Address,
    token_addr: Address,
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
    }
}

fn dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn params(ctx: &Ctx, pillar: Pillar, release_kind: ReleaseKind, cost: u32) -> CreateEventParams {
    CreateEventParams {
        pillar,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/bounty"),
        title: String::from_str(&ctx.env, "Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist(&ctx.env),
        application_credit_cost: cost,
        fee_bps_override: None,
    }
}

fn create_bounty(ctx: &Ctx, cost: u32) -> u64 {
    ctx.events.create_event(
        &params(ctx, Pillar::Bounty, ReleaseKind::Single, cost),
        &BytesN::random(&ctx.env),
    )
}

fn create_hackathon(ctx: &Ctx) -> u64 {
    ctx.events.create_event(
        &params(ctx, Pillar::Hackathon, ReleaseKind::Single, 0),
        &BytesN::random(&ctx.env),
    )
}

fn submit(ctx: &Ctx, bounty_id: u64) {
    let uri = String::from_str(&ctx.env, "ipfs://submission.json");
    ctx.events.submit(
        &bounty_id,
        &ctx.applicant,
        &uri,
        &BytesN::random(&ctx.env),
    );
}

fn complete_bounty(ctx: &Ctx, bounty_id: u64) {
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    ctx.events
        .select_winners(&bounty_id, &winners, &BytesN::random(&ctx.env));
}

fn contract_err<T>(result: Result<Result<T, Error>, soroban_sdk::Error>) -> Error {
    result.err().expect("contract call should fail").unwrap()
}

#[test]
fn bounty_create_requires_single_release_kind() {
    let ctx = setup();

    let err = contract_err(ctx.events.try_create_event(
        &params(&ctx, Pillar::Bounty, ReleaseKind::Multi(2), 1),
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(err, Error::InvalidReleaseKind);
}

#[test]
fn apply_bootstraps_profile_charges_credits_and_records_applicant() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 3);

    ctx.events.apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );

    let profile = ctx.profile.get_profile(&ctx.applicant).expect("profile");
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 3);
    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 1);
    assert_eq!(applicants.get(0).unwrap(), ctx.applicant);
}

#[test]
fn apply_rejects_missing_non_bounty_expired_duplicate_and_replayed_requests() {
    let ctx = setup();

    let missing = contract_err(ctx.events.try_apply_to_bounty(
        &999,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(missing, Error::EventNotFound);

    let hackathon_id = create_hackathon(&ctx);
    let non_bounty = contract_err(ctx.events.try_apply_to_bounty(
        &hackathon_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(non_bounty, Error::InvalidPillar);

    let bounty_id = create_bounty(&ctx, 1);
    ctx.env.ledger().with_mut(|li| {
        li.timestamp += 86_401;
    });
    let expired = contract_err(ctx.events.try_apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(expired, Error::DeadlinePassed);

    let fresh = create_bounty(&ctx, 1);
    let op = BytesN::random(&ctx.env);
    ctx.events.apply_to_bounty(&fresh, &ctx.applicant, &op);
    let replayed = contract_err(ctx.events.try_apply_to_bounty(&fresh, &ctx.applicant, &op));
    assert_eq!(replayed, Error::OpAlreadySeen);

    let duplicate = contract_err(ctx.events.try_apply_to_bounty(
        &fresh,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(duplicate, Error::ApplicantAlreadyApplied);
}

#[test]
fn apply_rejects_paused_contract_and_insufficient_credits() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);
    ctx.events.pause();

    let paused = contract_err(ctx.events.try_apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(paused, Error::Paused);

    ctx.events.unpause();
    let costly_id = create_bounty(&ctx, BOOTSTRAP_CREDITS + 1);
    let res = ctx.events.try_apply_to_bounty(
        &costly_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    assert!(res.is_err(), "profile spend should reject insufficient credits");
}

#[test]
fn withdraw_refunds_half_credit_cost_and_removes_applicant() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 5);

    ctx.events.apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    ctx.events.withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );

    let profile = ctx.profile.get_profile(&ctx.applicant).expect("profile");
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 5 + 2);
    assert_eq!(ctx.events.get_applicants(&bounty_id).len(), 0);
}

#[test]
fn withdraw_with_zero_cost_removes_applicant_without_credit_change() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    ctx.events.apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    let before = ctx.profile.get_profile(&ctx.applicant).expect("profile");
    ctx.events.withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    let after = ctx.profile.get_profile(&ctx.applicant).expect("profile");

    assert_eq!(after.credits, before.credits);
    assert_eq!(ctx.events.get_applicants(&bounty_id).len(), 0);
}

#[test]
fn withdraw_rejects_missing_non_bounty_not_applied_submitted_and_replayed_requests() {
    let ctx = setup();

    let missing = contract_err(ctx.events.try_withdraw_application(
        &999,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(missing, Error::EventNotFound);

    let hackathon_id = create_hackathon(&ctx);
    let non_bounty = contract_err(ctx.events.try_withdraw_application(
        &hackathon_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(non_bounty, Error::InvalidPillar);

    let bounty_id = create_bounty(&ctx, 1);
    let not_applied = contract_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(not_applied, Error::ApplicantNotApplied);

    ctx.events.apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    submit(&ctx, bounty_id);
    let submitted = contract_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(submitted, Error::SubmissionAlreadyExists);

    let fresh = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(
        &fresh,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    let op = BytesN::random(&ctx.env);
    ctx.events.withdraw_application(&fresh, &ctx.applicant, &op);
    let replayed = contract_err(ctx.events.try_withdraw_application(&fresh, &ctx.applicant, &op));
    assert_eq!(replayed, Error::OpAlreadySeen);
}

#[test]
fn withdraw_rejects_paused_expired_and_completed_events() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);
    ctx.events.apply_to_bounty(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    ctx.events.pause();

    let paused = contract_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(paused, Error::Paused);

    ctx.events.unpause();
    ctx.env.ledger().with_mut(|li| {
        li.timestamp += 86_401;
    });
    let expired = contract_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(expired, Error::DeadlinePassed);

    let fresh = create_bounty(&ctx, 0);
    ctx.events.apply_to_bounty(
        &fresh,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    );
    complete_bounty(&ctx, fresh);
    assert_eq!(ctx.events.get_event(&fresh).status, EventStatus::Completed);

    let completed = contract_err(ctx.events.try_withdraw_application(
        &fresh,
        &ctx.applicant,
        &BytesN::random(&ctx.env),
    ));
    assert_eq!(completed, Error::EventNotActive);
}
