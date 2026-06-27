// boundless-events: hackathon pillar tests.
//
// Covers the Pillar::Hackathon paths end-to-end against a real
// boundless-profile so the cross-contract credit / reputation / earnings
// side-effects of select_winners are exercised, not mocked:
//
//   - create_event validation: Hackathon requires ReleaseKind::Single and a
//     future deadline; the full budget is escrowed (fee taken at deposit).
//   - submit: open submission model — no prior apply, no credit charge.
//     deadline gate, re-submit, idempotency, withdraw.
//   - select_winners distribution: single-recipient sweep and multi-position
//     split. Each split test asserts BOTH recipient and fee-account deltas
//     (CLAUDE.md hard rule), plus profile bumps and the stored winner rows.
//   - select_winners rejections: empty set, position not in distribution,
//     duplicate position, replay (WinnersAlreadySelected), missing event,
//     already-completed event, and owner-auth requirement.
//   - claim_milestone is rejected for a Single-release hackathon.
//
// Spec: boundless-platform-contract-prd.md Section 7. Template:
// src/tests/crowdfunding.rs and src/tests/cross_contract.rs.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250;

// 10k USDC at 7 decimals.
const TOTAL_BUDGET: i128 = 10_000_0000000_i128;
const FEE_AMOUNT: i128 = (TOTAL_BUDGET * FEE_BPS as i128) / 10_000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    profile: ProfileContractClient<'a>,
    owner: Address,
    applicant: Address,
    token_addr: Address,
    fee_account: Address,
    events_admin: Address,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    // Non-root auth needed for token transfers and the cross-contract calls
    // into the profile contract during select_winners.
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

    // Touch fee_account's trustline (mint 0) and fund the owner.
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
        fee_account,
        events_admin,
    }
}

fn single_winner_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

// 50 / 30 / 20 across positions 1..=3.
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
        application_credit_cost: 0,
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

    // Owner paid budget + fee; fee account received the deposit-time fee.
    assert_eq!(owner_before - token.balance(&ctx.owner), TOTAL_BUDGET + FEE_AMOUNT);
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
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "hackathon must use Single release");
}

#[test]
fn create_rejects_missing_deadline() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Hackathon"),
        deadline: None,
        winner_distribution: single_winner_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "hackathon requires a submission deadline");
}

#[test]
fn create_rejects_past_deadline() {
    let ctx = setup();
    // try_ variant so we observe the error instead of panicking on the host.
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Hackathon"),
        // Equal-to-now is not in the future; create_event rejects it.
        deadline: Some(ctx.env.ledger().timestamp()),
        winner_distribution: single_winner_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "deadline must be in the future");
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
fn submit_after_deadline_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    // Jump the ledger past the 1-day submission deadline.
    ctx.env.ledger().with_mut(|li| {
        li.timestamp += 2 * 86_400;
    });

    let uri = String::from_str(&ctx.env, "ipfs://Qm.../late.json");
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_submit(&id, &ctx.applicant, &uri, &op);
    assert!(res.is_err(), "submission after the deadline must revert");
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
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    // Recipient delta: full budget. Fee account delta: 0 — the fee was taken
    // at deposit, never a second time on release.
    assert_eq!(token.balance(&ctx.applicant) - winner_before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);

    // Profile: fresh winner is bootstrapped then earns the win credits.
    let p = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(p.credits, BOOTSTRAP_CREDITS + 20);
    assert_eq!(p.reputation, 50);
    assert_eq!(
        ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr),
        TOTAL_BUDGET
    );

    // Escrow drained -> Completed; winner row recorded.
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
            credit_earn: 30,
            reputation_bump: 60,
        },
        WinnerSpec {
            recipient: second.clone(),
            position: 2,
            credit_earn: 20,
            reputation_bump: 40,
        },
        WinnerSpec {
            recipient: third.clone(),
            position: 3,
            credit_earn: 10,
            reputation_bump: 20,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let amt_1 = TOTAL_BUDGET * 50 / 100;
    let amt_2 = TOTAL_BUDGET * 30 / 100;
    let amt_3 = TOTAL_BUDGET * 20 / 100;

    // Recipient deltas across the split.
    assert_eq!(token.balance(&first), amt_1);
    assert_eq!(token.balance(&second), amt_2);
    assert_eq!(token.balance(&third), amt_3);
    // Fee account delta across the split: unchanged (no release-time fee).
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);

    // Profile bumps per winner.
    let p1 = ctx.profile.get_profile(&first).unwrap();
    let p2 = ctx.profile.get_profile(&second).unwrap();
    let p3 = ctx.profile.get_profile(&third).unwrap();
    assert_eq!(p1.credits, BOOTSTRAP_CREDITS + 30);
    assert_eq!(p1.reputation, 60);
    assert_eq!(p2.credits, BOOTSTRAP_CREDITS + 20);
    assert_eq!(p2.reputation, 40);
    assert_eq!(p3.credits, BOOTSTRAP_CREDITS + 10);
    assert_eq!(p3.reputation, 20);

    // 50 + 30 + 20 == 100 -> escrow fully drained -> Completed.
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
            credit_earn: 0,
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
            credit_earn: 0,
            reputation_bump: 0,
        },
        WinnerSpec {
            recipient: other,
            position: 1, // duplicate
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "duplicate position must revert");
}

#[test]
fn select_winners_second_call_reverts_winners_already_selected() {
    let ctx = setup();
    let dl = Some(ctx.env.ledger().timestamp() + 86_400);
    // 50/30/20 so the first call pays only position 1 and leaves the event
    // Active, isolating WinnersAlreadySelected from EventNotActive.
    let id = create_hackathon_with(&ctx, three_way_dist(&ctx.env), dl);

    let first_winner = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op1 = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &first_winner, &op1);

    // Event is still Active (60% escrow remains), but a prior anchor exists.
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);

    let second = Address::generate(&ctx.env);
    let second_winner = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: second,
            position: 2,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op2 = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &second_winner, &op2);
    assert!(res.is_err(), "a second select_winners must revert");
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
            credit_earn: 0,
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
            credit_earn: 0,
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
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);

    let again = Address::generate(&ctx.env);
    let more = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: again,
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op2 = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &more, &op2);
    assert!(res.is_err(), "select_winners on a Completed event must revert");
}

#[test]
fn select_winners_demands_owner_auth() {
    // mock_all_auths_allowing_non_root_auth lets the call succeed, but
    // env.auths() records which addresses had to authorize — the audit-relevant
    // observation. select_winners requires the event owner.
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: ctx.applicant.clone(),
            position: 1,
            credit_earn: 0,
            reputation_bump: 0,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let auths = ctx.env.auths();
    let owner_required = auths.iter().any(|(addr, _)| *addr == ctx.owner);
    assert!(owner_required, "select_winners must demand the event owner's auth");
    // Sanity: a random non-owner address was never asked to authorize.
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
        .try_claim_milestone(&id, &ctx.applicant, &0_u32, &0_u32, &0_u32, &op);
    assert!(
        res.is_err(),
        "claim_milestone must reject a Single-release hackathon"
    );
}
