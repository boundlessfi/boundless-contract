// Pull-model prize claims (#61): claim math, auth, replay/double-claim,
// the cancel gate + window escape hatch, and the pre-1.3.0 row decode guard.

#![cfg(test)]

use soroban_sdk::{
    contracttype,
    testutils::{Address as _, BytesN as _, Ledger as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::event_ops::PRIZE_CLAIM_WINDOW_SECS;
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
    token_addr: Address,
    fee_account: Address,
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

    Ctx {
        env,
        events,
        profile,
        owner,
        token_addr,
        fee_account,
    }
}

fn create_single(ctx: &Ctx, dist: Map<u32, i128>) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/prize-claim"),
        title: String::from_str(&ctx.env, "Prize Claim Suite"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        prize_floors: dist,
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

fn dist_100(env: &Env) -> Map<u32, i128> {
    let mut m = Map::new(env);
    m.set(1, TOTAL_BUDGET);
    m
}

fn dist_60_40(env: &Env) -> Map<u32, i128> {
    let mut m = Map::new(env);
    m.set(1, TOTAL_BUDGET * 60 / 100);
    m.set(2, TOTAL_BUDGET * 40 / 100);
    m
}

fn select_one(ctx: &Ctx, id: u64, recipient: &Address, position: u32, amount: i128, bump: u32) {
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: recipient.clone(),
            position,
            amount,
            reputation_bump: bump,
        },
    ];
    ctx.events
        .select_winners(&id, &winners, &BytesN::random(&ctx.env));
}

// ============================================================
// Claim math: recipient delta, fee delta, profile effects
// ============================================================

#[test]
fn claim_pays_recipient_full_prize_and_no_release_fee() {
    let ctx = setup();
    let id = create_single(&ctx, dist_100(&ctx.env));
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 50);

    // Selection alone must not move funds or complete the event.
    assert_eq!(token.balance(&w), 0);
    let mid = ctx.events.get_event(&id);
    assert_eq!(mid.status, EventStatus::Active);
    assert_eq!(mid.remaining_escrow, TOTAL_BUDGET);

    let fee_before = token.balance(&ctx.fee_account);
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));

    assert_eq!(token.balance(&w), TOTAL_BUDGET);
    assert_eq!(
        token.balance(&ctx.fee_account) - fee_before,
        0,
        "fee is charged at funding time, not at release"
    );

    let after = ctx.events.get_event(&id);
    assert_eq!(after.status, EventStatus::Completed);
    assert_eq!(after.remaining_escrow, 0);

    let p = ctx.profile.get_profile(&w).unwrap();
    assert_eq!(p.reputation, 50);
    assert_eq!(ctx.profile.get_earnings(&w, &ctx.token_addr), TOTAL_BUDGET);

    let rows = ctx.events.get_winners(&id);
    assert_eq!(rows.len(), 1);
    let row = rows.get(0).unwrap();
    assert_eq!(row.amount, TOTAL_BUDGET);
    assert!(row.paid_at.is_some());
}

#[test]
fn split_claims_pay_exact_amounts_each() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let a = Address::generate(&ctx.env);
    let b = Address::generate(&ctx.env);
    select_one(&ctx, id, &a, 1, TOTAL_BUDGET * 60 / 100, 10);
    select_one(&ctx, id, &b, 2, TOTAL_BUDGET * 40 / 100, 5);

    ctx.events
        .claim_prize(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(token.balance(&b), TOTAL_BUDGET * 40 / 100);
    assert_eq!(
        ctx.events.get_event(&id).status,
        EventStatus::Active,
        "event stays active until every prize is claimed"
    );

    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert_eq!(token.balance(&a), TOTAL_BUDGET * 60 / 100);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Completed);
}

#[test]
fn topup_after_selection_stays_residual_for_refund() {
    let ctx = setup();
    let id = create_single(&ctx, dist_100(&ctx.env));
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let token_admin = token::StellarAssetClient::new(&ctx.env, &ctx.token_addr);

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 0);

    // A partner tops up AFTER selection: the prize amount stays anchored
    // to the selection-time baseline; the top-up is refundable residual.
    let p = Address::generate(&ctx.env);
    let c = 500_0000000_i128;
    let fee = c * FEE_BPS as i128 / 10_000_i128;
    token_admin.mint(&p, &(c + fee));
    ctx.events.add_funds(&id, &p, &c, &BytesN::random(&ctx.env));

    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert_eq!(token.balance(&w), TOTAL_BUDGET);

    let after = ctx.events.get_event(&id);
    assert_eq!(after.status, EventStatus::Active);
    assert_eq!(after.remaining_escrow, c);

    // With every prize claimed, the manager can cancel and the partner
    // gets their contribution back.
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(token.balance(&p), c);
}

// ============================================================
// Auth surface
// ============================================================

#[test]
fn claim_requires_recipient_auth() {
    let ctx = setup();
    let id = create_single(&ctx, dist_100(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 0);

    ctx.env.mock_auths(&[]);
    let res = ctx
        .events
        .try_claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert!(res.is_err(), "claim without the winner's auth must revert");
}

#[test]
fn claim_demands_the_award_recipients_auth_specifically() {
    let ctx = setup();
    let id = create_single(&ctx, dist_100(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 0);

    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == w),
        "the recorded recipient must be the authorizing address"
    );
}

// ============================================================
// Replay and not-found guards
// ============================================================

#[test]
fn double_claim_reverts() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET * 60 / 100, 0);

    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    let res = ctx
        .events
        .try_claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert!(
        res.is_err(),
        "second claim of the same position must revert"
    );
}

#[test]
fn op_id_replay_reverts() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));

    // Both positions go to the same recipient so the two claims share an
    // op_id domain; reusing the op_id must revert even though position 2 is
    // not yet paid. (Cross-recipient op_id reuse is intentionally allowed —
    // OpSeen is namespaced per authorizing caller.)
    let a = Address::generate(&ctx.env);
    select_one(&ctx, id, &a, 1, TOTAL_BUDGET * 60 / 100, 0);
    select_one(&ctx, id, &a, 2, TOTAL_BUDGET * 40 / 100, 0);

    let op = BytesN::random(&ctx.env);
    ctx.events.claim_prize(&id, &1_u32, &op);
    let res = ctx.events.try_claim_prize(&id, &2_u32, &op);
    assert!(
        res.is_err(),
        "replaying an op_id in the same domain must revert"
    );
}

#[test]
fn claim_of_unawarded_position_reverts() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET * 60 / 100, 0);

    // Position 2 is in the distribution but has no award yet.
    let res = ctx
        .events
        .try_claim_prize(&id, &2_u32, &BytesN::random(&ctx.env));
    assert!(res.is_err(), "claiming an unawarded position must revert");

    // Position 9 is not even in the distribution.
    let res = ctx
        .events
        .try_claim_prize(&id, &9_u32, &BytesN::random(&ctx.env));
    assert!(res.is_err(), "claiming an unknown position must revert");
}

#[test]
fn claim_on_multi_release_event_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(2),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Grant"),
        deadline: None,
        prize_floors: dist_100(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 0);

    let res = ctx
        .events
        .try_claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert!(
        res.is_err(),
        "claim_prize on a Multi-release event must revert"
    );
}

// ============================================================
// Cancel gate and the claim-window escape hatch
// ============================================================

#[test]
fn cancel_blocked_while_unclaimed_prizes_within_window() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET * 60 / 100, 0);

    let res = ctx.events.try_start_cancel(&id, &BytesN::random(&ctx.env));
    assert!(
        res.is_err(),
        "cancel must be blocked while prizes are unclaimed in-window"
    );

    // Once the winner claims, cancellation may sweep the residual 40%.
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    ctx.events
        .claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    let owner_before = token.balance(&ctx.owner);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(
        token.balance(&ctx.owner) - owner_before,
        TOTAL_BUDGET * 40 / 100
    );
}

#[test]
fn cancel_after_window_expiry_sweeps_unclaimed_and_blocks_late_claim() {
    let ctx = setup();
    let id = create_single(&ctx, dist_100(&ctx.env));
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let w = Address::generate(&ctx.env);
    select_one(&ctx, id, &w, 1, TOTAL_BUDGET, 0);

    ctx.env.ledger().with_mut(|li| {
        li.timestamp += PRIZE_CLAIM_WINDOW_SECS + 1;
    });

    let owner_before = token.balance(&ctx.owner);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(
        token.balance(&ctx.owner) - owner_before,
        TOTAL_BUDGET,
        "expired unclaimed prize sweeps back through the refund path"
    );

    let res = ctx
        .events
        .try_claim_prize(&id, &1_u32, &BytesN::random(&ctx.env));
    assert!(res.is_err(), "claim after cancellation must revert");
    assert_eq!(token.balance(&w), 0);
}

#[test]
fn claim_window_refreshes_on_a_later_batch() {
    let ctx = setup();
    let id = create_single(&ctx, dist_60_40(&ctx.env));

    let a = Address::generate(&ctx.env);
    select_one(&ctx, id, &a, 1, TOTAL_BUDGET * 60 / 100, 0);

    // Move to just before the first window expires, then select batch 2.
    ctx.env.ledger().with_mut(|li| {
        li.timestamp += PRIZE_CLAIM_WINDOW_SECS - 100;
    });
    let b = Address::generate(&ctx.env);
    select_one(&ctx, id, &b, 2, TOTAL_BUDGET * 40 / 100, 0);

    // Past the FIRST batch's expiry, but inside the refreshed window:
    // cancel stays blocked, protecting the late-selected winner.
    ctx.env.ledger().with_mut(|li| {
        li.timestamp += 200;
    });
    let res = ctx.events.try_start_cancel(&id, &BytesN::random(&ctx.env));
    assert!(
        res.is_err(),
        "refreshed window must keep protecting unclaimed winners"
    );
}

// ============================================================
// Storage-layout guard for pre-1.3.0 winner rows
// ============================================================

// Winner as persisted up to 1.2.0. Adding a field to types::Winner breaks
// this test (UnexpectedSize) because deployed rows would fail to decode on
// upgrade. Persist new per-winner data under new DataKeys instead.
#[contracttype]
#[derive(Clone, Debug)]
pub struct WinnerRowV1 {
    pub recipient: Address,
    pub position: u32,
    pub amount: i128,
    pub milestone: Option<u32>,
    pub paid_at: Option<u64>,
}

#[test]
fn pre_upgrade_winner_rows_still_decode() {
    let ctx = setup();
    let recipient = Address::generate(&ctx.env);

    ctx.env.as_contract(&ctx.events.address, || {
        ctx.env.storage().persistent().set(
            &DataKey::EventWinnerAt(7u64, 0u32),
            &WinnerRowV1 {
                recipient: recipient.clone(),
                position: 1,
                amount: 1_000,
                milestone: None,
                paid_at: Some(12_345),
            },
        );
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::EventWinnerCount(7u64), &1u32);
    });

    let row = ctx.env.as_contract(&ctx.events.address, || {
        crate::storage::winner_at(&ctx.env, 7, 0)
    });
    assert!(
        row.is_some(),
        "pre-1.3.0 Winner rows must stay readable after upgrade"
    );
    assert_eq!(row.unwrap().recipient, recipient);
}
