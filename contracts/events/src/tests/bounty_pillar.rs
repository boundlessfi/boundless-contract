// boundless-events: bounty pillar tests.
//
// Covers Pillar::Bounty paths:
//   - validate_create (Single release only; application_credit_cost cap)
//   - apply_to_bounty (credit bootstrap + spend via profile)
//   - withdraw_application (50% credit refund)
//   - auth, idempotency, pause, deadline, and lifecycle guards
//
// Spec: boundless-platform-contract-prd.md Sections 6.3, 7.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
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
    let profile_id = env.register(ProfileContract, (profile_admin.clone(), BOOTSTRAP_CREDITS));
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
    }
}

fn one_winner_distribution(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_bounty(ctx: &Ctx, application_credit_cost: u32) -> u64 {
    create_bounty_with_deadline(
        ctx,
        application_credit_cost,
        ctx.env.ledger().timestamp() + 86_400,
    )
}

fn create_bounty_with_deadline(ctx: &Ctx, application_credit_cost: u32, deadline: u64) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/events/draft/x"),
        title: String::from_str(&ctx.env, "Test Bounty"),
        deadline: Some(deadline),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost,
        fee_bps_override: None,
        manager: None,
    };
    let op_id = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op_id)
}

fn create_hackathon(ctx: &Ctx) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Test Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn expect_op_err<T, E>(
    result: Result<Result<T, E>, Result<Error, soroban_sdk::InvokeError>>,
) -> Error {
    match result {
        Err(Ok(e)) => e,
        _ => panic!("expected contract error"),
    }
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn create_rejects_multi_release_kind() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_create_event(&params, &op));
    assert_eq!(err, Error::InvalidReleaseKind);
}

#[test]
fn create_rejects_excessive_application_credit_cost() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: one_winner_distribution(&ctx.env),
        application_credit_cost: 101,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_create_event(&params, &op));
    assert_eq!(err, Error::InvalidPillar);
}

// ============================================================
// apply_to_bounty — happy path + credits
// ============================================================

#[test]
fn apply_charges_credits_via_profile() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    assert!(ctx.profile.get_profile(&ctx.applicant).is_none());

    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    let profile = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("bootstrapped");
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS - 1);

    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 1);
    assert_eq!(applicants.get(0).unwrap(), ctx.applicant);
}

#[test]
fn apply_with_zero_credit_cost_bootstraps_without_spending() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    let profile = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("bootstrapped");
    assert_eq!(profile.credits, BOOTSTRAP_CREDITS);
}

// ============================================================
// apply_to_bounty — errors + idempotency
// ============================================================

#[test]
fn duplicate_apply_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let op_a = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_a);

    let op_b = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_b),
    );
    assert_eq!(err, Error::ApplicantAlreadyApplied);
}

#[test]
fn insufficient_credits_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 100);

    let op_id = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);
    assert!(res.is_err(), "profile InsufficientCredits should bubble up");
}

#[test]
fn replayed_apply_reverts_idempotently() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn apply_on_nonexistent_event_reverts() {
    let ctx = setup();
    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&9999, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::EventNotFound);
}

#[test]
fn apply_on_wrong_pillar_reverts() {
    let ctx = setup();
    let hackathon_id = create_hackathon(&ctx);
    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&hackathon_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::InvalidPillar);
}

#[test]
fn apply_on_cancelled_event_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);
    drive_cancel(&ctx.env, &ctx.events, bounty_id);

    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::EventNotActive);
}

#[test]
fn apply_on_completed_event_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

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

    let event = ctx.events.get_event(&bounty_id);
    assert_eq!(event.status, EventStatus::Completed);

    let op_retry = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_retry),
    );
    assert_eq!(err, Error::EventNotActive);
}

#[test]
fn apply_after_deadline_reverts() {
    let ctx = setup();
    let deadline = ctx.env.ledger().timestamp() + 100;
    let bounty_id = create_bounty_with_deadline(&ctx, 1, deadline);

    ctx.env.ledger().with_mut(|li| {
        li.timestamp = deadline;
    });

    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::DeadlinePassed);
}

#[test]
fn apply_when_paused_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);
    ctx.events.pause();

    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::Paused);
}

#[test]
fn apply_requires_applicant_auth() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);
    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    let auths = ctx.env.auths();
    let applicant_required = auths.iter().any(|(addr, _)| *addr == ctx.applicant);
    assert!(applicant_required, "apply must demand applicant auth");
}

// ============================================================
// withdraw_application — happy path + credits
// ============================================================

#[test]
fn withdraw_refunds_half_credits() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 2);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let after_apply = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("bootstrapped");
    assert_eq!(after_apply.credits, BOOTSTRAP_CREDITS - 2);

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_application(&bounty_id, &ctx.applicant, &op_wd);

    let after_wd = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("still exists");
    assert_eq!(after_wd.credits, BOOTSTRAP_CREDITS - 2 + 1);

    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 0);
}

#[test]
fn withdraw_with_zero_credit_cost_skips_refund() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 0);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let before = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("bootstrapped")
        .credits;

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_application(&bounty_id, &ctx.applicant, &op_wd);

    let after = ctx
        .profile
        .get_profile(&ctx.applicant)
        .expect("still exists");
    assert_eq!(after.credits, before);

    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 0);
}

// ============================================================
// withdraw_application — errors + idempotency
// ============================================================

#[test]
fn withdraw_without_apply_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);
    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &op_id,
    ));
    assert_eq!(err, Error::ApplicantNotApplied);
}

#[test]
fn withdraw_after_submit_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../bounty.json");
    let op_submit = BytesN::random(&ctx.env);
    ctx.events
        .submit(&bounty_id, &ctx.applicant, &uri, &op_submit);

    let op_wd = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &op_wd,
    ));
    assert_eq!(err, Error::SubmissionAlreadyExists);
}

#[test]
fn replayed_withdraw_reverts_idempotently() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 2);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_application(&bounty_id, &ctx.applicant, &op_wd);

    let err = expect_op_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &op_wd,
    ));
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn withdraw_on_nonexistent_event_reverts() {
    let ctx = setup();
    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_withdraw_application(&9999, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::EventNotFound);
}

#[test]
fn withdraw_when_paused_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    ctx.events.pause();

    let op_wd = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_withdraw_application(
        &bounty_id,
        &ctx.applicant,
        &op_wd,
    ));
    assert_eq!(err, Error::Paused);
}

#[test]
fn withdraw_requires_applicant_auth() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx, 1);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_application(&bounty_id, &ctx.applicant, &op_wd);

    let auths = ctx.env.auths();
    let applicant_required = auths.iter().any(|(addr, _)| *addr == ctx.applicant);
    assert!(applicant_required, "withdraw must demand applicant auth");
}
