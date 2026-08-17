#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::errors::Error;
use crate::types::{CreateEventParams, DataKey, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

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
    }
}

fn one_winner_distribution(env: &Env) -> Map<u32, i128> {
    let mut m = Map::new(env);
    m.set(1, 100000000000_i128);
    m
}

fn create_bounty(ctx: &Ctx) -> u64 {
    create_bounty_with_deadline(ctx, ctx.env.ledger().timestamp() + 86_400)
}

fn create_bounty_with_deadline(ctx: &Ctx, deadline: u64) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/events/draft/x"),
        title: String::from_str(&ctx.env, "Test Bounty"),
        deadline: Some(deadline),
        prize_floors: one_winner_distribution(&ctx.env),
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
        prize_floors: one_winner_distribution(&ctx.env),
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
        prize_floors: one_winner_distribution(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_create_event(&params, &op));
    assert_eq!(err, Error::InvalidReleaseKind);
}

// ============================================================
// apply_to_bounty — happy path
// ============================================================

#[test]
fn apply_bootstraps_profile_and_records_applicant() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    assert!(ctx.profile.get_profile(&ctx.applicant).is_none());

    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    assert!(ctx.profile.get_profile(&ctx.applicant).is_some());

    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 1);
    assert_eq!(applicants.get(0).unwrap(), ctx.applicant);
}

// ============================================================
// apply_to_bounty — errors + idempotency
// ============================================================

#[test]
fn duplicate_apply_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

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
fn replayed_apply_reverts_idempotently() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

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
    let bounty_id = create_bounty(&ctx);
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
    let bounty_id = create_bounty(&ctx);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            amount: 10_000_0000000_i128,
            reputation_bump: 0,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Pull model: claim the sole prize to drain escrow and complete.
    ctx.events
        .claim_prize(&bounty_id, &1_u32, &BytesN::random(&ctx.env));

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
fn apply_when_paused_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);
    ctx.events.pause();

    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(err, Error::Paused);
}

#[test]
fn apply_beyond_former_cap_succeeds() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    // Fast-forward the per-event counter past the former 5,000 cap instead
    // of performing that many real applications from distinct addresses.
    ctx.env.as_contract(&ctx.events.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::EventApplicantCount(bounty_id), &5_000_u32);
    });

    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    assert_eq!(
        ctx.events.get_applicant_count(&bounty_id),
        5_001,
        "applications are unbounded; the counter must keep advancing past the former cap"
    );
    assert_eq!(
        ctx.events.get_applicant_at(&bounty_id, &5_000),
        Some(ctx.applicant.clone()),
        "the new applicant must land in the next slot"
    );
}

#[test]
fn apply_at_counter_overflow_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    ctx.env.as_contract(&ctx.events.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::EventApplicantCount(bounty_id), &u32::MAX);
    });

    let op_id = BytesN::random(&ctx.env);
    let err = expect_op_err(
        ctx.events
            .try_apply_to_bounty(&bounty_id, &ctx.applicant, &op_id),
    );
    assert_eq!(
        err,
        Error::TooManyApplicants,
        "an application that would overflow the u32 counter must revert"
    );
}

#[test]
fn applicants_page_respects_start_and_limit() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let second = Address::generate(&ctx.env);
    let third = Address::generate(&ctx.env);
    for applicant in [&ctx.applicant, &second, &third] {
        ctx.events
            .apply_to_bounty(&bounty_id, applicant, &BytesN::random(&ctx.env));
    }

    let tail = ctx.events.get_applicants_page(&bounty_id, &1, &10);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail.get(0).unwrap(), second);
    assert_eq!(tail.get(1).unwrap(), third);

    assert_eq!(
        ctx.events.get_applicants_page(&bounty_id, &0, &0).len(),
        0,
        "limit 0 must return an empty page"
    );
    assert_eq!(
        ctx.events.get_applicants_page(&bounty_id, &10, &5).len(),
        0,
        "a start past the end must return an empty page"
    );
    assert_eq!(
        ctx.events.get_applicants_page(&bounty_id, &0, &1_000).len(),
        3,
        "an oversized limit is clamped, not an error"
    );
}

#[test]
fn apply_requires_applicant_auth() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);
    let op_id = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_id);

    let auths = ctx.env.auths();
    let applicant_required = auths.iter().any(|(addr, _)| *addr == ctx.applicant);
    assert!(applicant_required, "apply must demand applicant auth");
}

// ============================================================
// withdraw_application — happy path
// ============================================================

#[test]
fn withdraw_removes_the_applicant() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);

    let op_apply = BytesN::random(&ctx.env);
    ctx.events
        .apply_to_bounty(&bounty_id, &ctx.applicant, &op_apply);
    assert_eq!(ctx.events.get_applicants(&bounty_id).len(), 1);

    let op_wd = BytesN::random(&ctx.env);
    ctx.events
        .withdraw_application(&bounty_id, &ctx.applicant, &op_wd);

    let applicants = ctx.events.get_applicants(&bounty_id);
    assert_eq!(applicants.len(), 0);
}

// ============================================================
// withdraw_application — errors + idempotency
// ============================================================

#[test]
fn withdraw_without_apply_reverts() {
    let ctx = setup();
    let bounty_id = create_bounty(&ctx);
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
    let bounty_id = create_bounty(&ctx);

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
    let bounty_id = create_bounty(&ctx);

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
    let bounty_id = create_bounty(&ctx);

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
    let bounty_id = create_bounty(&ctx);

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
