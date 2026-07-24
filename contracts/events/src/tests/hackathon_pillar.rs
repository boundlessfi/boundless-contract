#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::errors::Error;
use crate::event_ops::{MAX_CONTENT_URI_LEN, MAX_SUBMISSIONS_PER_EVENT};
use crate::storage;
use crate::types::{CreateEventParams, DataKey, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const FEE_BPS: u32 = 250;

const TOTAL_BUDGET: i128 = 10_000_0000000_i128;
const FEE_AMOUNT: i128 = (TOTAL_BUDGET * FEE_BPS as i128) / 10_000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    events_id: Address,
    profile: ProfileContractClient<'a>,
    owner: Address,
    applicant: Address,
    token_addr: Address,
    fee_account: Address,
    events_admin: Address,
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
        events_id,
        profile,
        owner,
        applicant,
        token_addr,
        fee_account,
        events_admin,
    }
}

fn expect_op_err<T, E>(
    result: Result<Result<T, E>, Result<Error, soroban_sdk::InvokeError>>,
) -> Error {
    match result {
        Err(Ok(e)) => e,
        _ => panic!("expected contract error"),
    }
}

fn single_winner_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn three_way_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 50);
    m.set(2, 30);
    m.set(3, 20);
    m
}

fn create_hackathon_with(ctx: &Ctx, dist: Map<u32, u32>, deadline: Option<u64>) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/hackathon"),
        title: String::from_str(&ctx.env, "Test Hackathon"),
        deadline,
        winner_distribution: dist,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn create_hackathon(ctx: &Ctx) -> u64 {
    let dl = Some(ctx.env.ledger().timestamp() + 86_400);
    create_hackathon_with(ctx, single_winner_dist(&ctx.env), dl)
}

// ============================================================
// create_event / validate_create
// ============================================================

#[test]
fn create_deposits_full_budget_and_takes_fee_at_deposit() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    let id = create_hackathon(&ctx);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Hackathon);
    assert_eq!(event.status, EventStatus::Active);
    assert_eq!(
        event.remaining_escrow, TOTAL_BUDGET,
        "hackathon escrows the full budget at create"
    );

    assert_eq!(
        owner_before - token.balance(&ctx.owner),
        TOTAL_BUDGET + FEE_AMOUNT
    );
    assert_eq!(token.balance(&ctx.fee_account), FEE_AMOUNT);
}

#[test]
fn create_rejects_multi_release_kind() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad Hackathon"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_winner_dist(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "hackathon must use Single release");
}

// ============================================================
// submit (open submission model)
// ============================================================

#[test]
fn submit_open_without_prior_apply_creates_anchor() {
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
fn resubmit_keeps_original_timestamp_and_updates_uri() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri_a = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op_a = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri_a, &op_a);
    let first_time = ctx.events.get_submission(&id, &ctx.applicant).submitted_at;

    let uri_b = String::from_str(&ctx.env, "ipfs://Qm.../v2.json");
    let op_b = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri_b, &op_b);

    let second = ctx.events.get_submission(&id, &ctx.applicant);
    assert_eq!(second.content_uri, uri_b);
    assert_eq!(second.submitted_at, first_time);
}

#[test]
fn submit_replayed_op_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op);

    let res = ctx.events.try_submit(&id, &ctx.applicant, &uri, &op);
    assert!(res.is_err(), "replayed submit op_id must revert");
}

#[test]
fn withdraw_submission_removes_anchor() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    let op_s = BytesN::random(&ctx.env);
    ctx.events.submit(&id, &ctx.applicant, &uri, &op_s);

    let op_w = BytesN::random(&ctx.env);
    ctx.events.withdraw_submission(&id, &ctx.applicant, &op_w);

    let res = ctx.events.try_get_submission(&id, &ctx.applicant);
    assert!(res.is_err(), "withdrawn submission is no longer readable");
}

#[test]
fn remove_submission_on_nonexistent_entry_does_not_corrupt_counter() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let submitter = Address::generate(&ctx.env);
    ctx.events.submit(
        &id,
        &submitter,
        &String::from_str(&ctx.env, "ipfs://Qm.../v1.json"),
        &BytesN::random(&ctx.env),
    );

    // ctx.applicant never submitted — calling the low-level storage helper
    // directly for it must be a no-op, not decrement the counter that
    // `submitter`'s real submission incremented.
    ctx.env.as_contract(&ctx.events_id, || {
        storage::remove_submission(&ctx.env, id, &ctx.applicant);
    });

    let count = ctx
        .env
        .as_contract(&ctx.events_id, || storage::submission_count(&ctx.env, id));
    assert_eq!(
        count, 1,
        "removing a nonexistent submission must not corrupt the counter"
    );
}

#[test]
fn withdraw_submission_frees_the_slot_for_future_submitters() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    ctx.events
        .submit(&id, &ctx.applicant, &uri, &BytesN::random(&ctx.env));
    ctx.events
        .withdraw_submission(&id, &ctx.applicant, &BytesN::random(&ctx.env));

    let count = ctx
        .env
        .as_contract(&ctx.events_id, || storage::submission_count(&ctx.env, id));
    assert_eq!(
        count, 0,
        "withdrawing a submission must free its slot against the cap"
    );
}

#[test]
fn submit_beyond_cap_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    // Fast-forward the per-event counter directly instead of performing
    // MAX_SUBMISSIONS_PER_EVENT real submissions from distinct addresses.
    ctx.env.as_contract(&ctx.events_id, || {
        ctx.env.storage().persistent().set(
            &DataKey::EventSubmissionCount(id),
            &MAX_SUBMISSIONS_PER_EVENT,
        );
    });

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../overflow.json");
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_submit(&id, &ctx.applicant, &uri, &op));
    assert_eq!(
        err,
        Error::TooManyContributors,
        "a submission at cap + 1 must revert"
    );
}

#[test]
fn submit_oversized_content_uri_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let too_long = "x".repeat((MAX_CONTENT_URI_LEN + 1) as usize);
    let uri = String::from_str(&ctx.env, &too_long);
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_submit(&id, &ctx.applicant, &uri, &op));
    // Reused rather than a new variant — stays inside the contracterror
    // 50-variant cap (see BACKLOG.md L7 for precedent).
    assert_eq!(
        err,
        Error::TitleTooLong,
        "content_uri beyond MAX_CONTENT_URI_LEN must revert"
    );
}

#[test]
fn resubmit_by_existing_applicant_does_not_increment_submission_count() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let uri_a = String::from_str(&ctx.env, "ipfs://Qm.../v1.json");
    ctx.events
        .submit(&id, &ctx.applicant, &uri_a, &BytesN::random(&ctx.env));

    let count_after_first = ctx
        .env
        .as_contract(&ctx.events_id, || storage::submission_count(&ctx.env, id));
    assert_eq!(count_after_first, 1);

    let uri_b = String::from_str(&ctx.env, "ipfs://Qm.../v2.json");
    ctx.events
        .submit(&id, &ctx.applicant, &uri_b, &BytesN::random(&ctx.env));

    let count_after_second = ctx
        .env
        .as_contract(&ctx.events_id, || storage::submission_count(&ctx.env, id));
    assert_eq!(
        count_after_second, 1,
        "re-submission by an existing applicant updates in place and must not \
         recount against the cap"
    );
}

// ============================================================
// select_winners — distribution (happy paths)
// ============================================================

#[test]
fn select_winners_single_recipient_sweeps_escrow() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let winner_before = token.balance(&ctx.applicant);
    let fee_before = token.balance(&ctx.fee_account);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 50,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    // Pull model: selection only records; the winner claims in their own tx.
    assert_eq!(token.balance(&ctx.applicant) - winner_before, 0);
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&ctx.applicant) - winner_before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);

    let p = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(p.reputation, 50);
    assert_eq!(
        ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr),
        TOTAL_BUDGET
    );

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);

    let winner_list = ctx.events.get_winners(&id);
    assert_eq!(winner_list.len(), 1);
    let w = winner_list.get(0).unwrap();
    assert_eq!(w.recipient, ctx.applicant);
    assert_eq!(w.position, 1);
    assert_eq!(w.amount, TOTAL_BUDGET);
    assert!(w.milestone.is_none());
    assert!(w.paid_at.is_some());
}

#[test]
fn select_winners_multi_position_splits_by_distribution() {
    let ctx = setup();
    let dl = Some(ctx.env.ledger().timestamp() + 86_400);
    let id = create_hackathon_with(&ctx, three_way_dist(&ctx.env), dl);

    let first = Address::generate(&ctx.env);
    let second = Address::generate(&ctx.env);
    let third = Address::generate(&ctx.env);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: first.clone(),
            position: 1,
            reputation_bump: 60,
        },
        WinnerSpec {
            recipient: second.clone(),
            position: 2,
            reputation_bump: 40,
        },
        WinnerSpec {
            recipient: third.clone(),
            position: 3,
            reputation_bump: 20,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    // Pull model: each winner claims their own position.
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    ctx.events
        .claim_prize(&id, &2_u32, &BytesN::random(&ctx.env));
    ctx.events
        .claim_prize(&id, &3_u32, &BytesN::random(&ctx.env));

    let amt_1 = TOTAL_BUDGET * 50 / 100;
    let amt_2 = TOTAL_BUDGET * 30 / 100;
    let amt_3 = TOTAL_BUDGET * 20 / 100;

    assert_eq!(token.balance(&first), amt_1);
    assert_eq!(token.balance(&second), amt_2);
    assert_eq!(token.balance(&third), amt_3);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);

    let p1 = ctx.profile.get_profile(&first).unwrap();
    let p2 = ctx.profile.get_profile(&second).unwrap();
    let p3 = ctx.profile.get_profile(&third).unwrap();
    assert_eq!(p1.reputation, 60);
    assert_eq!(p2.reputation, 40);
    assert_eq!(p3.reputation, 20);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Completed);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(ctx.events.get_winners(&id).len(), 3);
}

// ============================================================
// select_winners — rejections / edges
// ============================================================

#[test]
fn select_winners_empty_set_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![&ctx.env];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "empty winner set must revert");
}

#[test]
fn select_winners_position_not_in_distribution_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx); // distribution only has position 1

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 2,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "position outside distribution must revert");
}

#[test]
fn select_winners_duplicate_position_reverts() {
    let ctx = setup();
    let dl = Some(ctx.env.ledger().timestamp() + 86_400);
    let id = create_hackathon_with(&ctx, three_way_dist(&ctx.env), dl);

    let other = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
        WinnerSpec {
            recipient: other,
            position: 1, // duplicate
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "duplicate position must revert");
}

#[test]
fn select_winners_batches_append_and_position_replay_reverts() {
    let ctx = setup();
    let dl = Some(ctx.env.ledger().timestamp() + 86_400);
    let id = create_hackathon_with(&ctx, three_way_dist(&ctx.env), dl);

    let first_winner = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op1 = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &first_winner, &op1);

    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);

    // Re-awarding an already-taken position must revert, even across calls.
    let usurper = Address::generate(&ctx.env);
    let replay = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: usurper,
            position: 1,
            reputation_bump: 0,
        },
    ];
    let res = ctx
        .events
        .try_select_winners(&id, &replay, &BytesN::random(&ctx.env));
    assert!(res.is_err(), "re-awarding a taken position must revert");

    // A later batch for untaken positions appends (1.3.0 batching), and
    // amounts stay anchored to the baseline captured at the first batch.
    let second = Address::generate(&ctx.env);
    let third = Address::generate(&ctx.env);
    let batch2 = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: second.clone(),
            position: 2,
            reputation_bump: 0,
        },
        WinnerSpec {
            recipient: third.clone(),
            position: 3,
            reputation_bump: 0,
        },
    ];
    ctx.events
        .select_winners(&id, &batch2, &BytesN::random(&ctx.env));

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    ctx.events
        .claim_prize(&id, &2_u32, &BytesN::random(&ctx.env));
    ctx.events
        .claim_prize(&id, &3_u32, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET * 50 / 100);
    assert_eq!(token.balance(&second), TOTAL_BUDGET * 30 / 100);
    assert_eq!(token.balance(&third), TOTAL_BUDGET * 20 / 100);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);
}

#[test]
fn select_winners_replayed_op_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "replayed select_winners op_id must revert");
}

#[test]
fn select_winners_on_missing_event_reverts() {
    let ctx = setup();
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&404_u64, &winners, &op);
    assert!(res.is_err(), "unknown event id must revert");
}

#[test]
fn select_winners_on_completed_event_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx); // 100% to one winner -> Completed

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);
    // Pull model: the event completes when the last prize is claimed.
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);

    let again = Address::generate(&ctx.env);
    let more = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: again,
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op2 = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &more, &op2);
    assert!(
        res.is_err(),
        "select_winners on a Completed event must revert"
    );
}

#[test]
fn select_winners_demands_owner_auth() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let auths = ctx.env.auths();
    let owner_required = auths.iter().any(|(addr, _)| *addr == ctx.owner);
    assert!(
        owner_required,
        "select_winners must demand the event owner's auth"
    );
    assert!(
        !auths.iter().any(|(addr, _)| *addr == ctx.events_admin),
        "the events admin is not an authorizer of select_winners"
    );
}

// ============================================================
// claim_milestone is not a hackathon path
// ============================================================

#[test]
fn claim_milestone_on_single_release_hackathon_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&id, &ctx.applicant, &0_u32, &0_u32, &op);
    assert!(
        res.is_err(),
        "claim_milestone must reject a Single-release hackathon"
    );
}
