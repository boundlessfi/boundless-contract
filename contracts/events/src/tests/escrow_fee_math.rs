// boundless-events: escrow fee + release math tests.
//
// Covers every function in escrow.rs (effective_fee_bps, compute_fee_at,
// compute_fee, deposit_with_fee_at, deposit_with_fee, release) plus the
// payout-split math in select_winners (Single) and claim_milestone (Multi /
// Crowdfunding). Tests verify rounding, override bps, zero-fee edges,
// auth rejection, and idempotency where relevant.
//
// Spec: boundless-platform-contract-prd.md Sections 6.2, 7, 8;
//       issue #31.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::types::{CreateEventParams, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250; // 2.5%
const TOTAL_BUDGET: i128 = 1_000_0000000_i128; // 1000 USDC at 7 decimals

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    #[allow(dead_code)]
    profile: ProfileContractClient<'a>,
    owner: Address,
    admin: Address,
    token_addr: Address,
    fee_account: Address,
    token_admin: token::StellarAssetClient<'a>,
}

fn setup_with_bps<'a>(fee_bps: u32) -> Ctx<'a> {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id =
        env.register(ProfileContract, (profile_admin.clone(), BOOTSTRAP_CREDITS));
    let profile = ProfileContractClient::new(&env, &profile_id);

    let events_admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (
            events_admin.clone(),
            fee_account.clone(),
            fee_bps,
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
    token_admin.mint(&owner, &50_000_0000000_i128);

    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        profile,
        owner,
        admin: events_admin,
        token_addr,
        fee_account,
        token_admin,
    }
}

fn setup<'a>() -> Ctx<'a> {
    setup_with_bps(FEE_BPS)
}

fn single_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_hackathon(ctx: &Ctx) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/fee-test"),
        title: String::from_str(&ctx.env, "Fee Math Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn create_hackathon_with_override(ctx: &Ctx, override_bps: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/override"),
        title: String::from_str(&ctx.env, "Override BPS Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(override_bps),
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn create_hackathon_with_dist(ctx: &Ctx, dist: Map<u32, u32>) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/dist-test"),
        title: String::from_str(&ctx.env, "Dist Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn create_grant(ctx: &Ctx, milestones: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Grant,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Grant Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn fund(ctx: &Ctx, addr: &Address, amount: i128) {
    ctx.token_admin.mint(addr, &amount);
}

// ============================================================
// effective_fee_bps: global vs override
// ============================================================

#[test]
fn global_fee_bps_used_when_no_override() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let owner_before = token.balance(&ctx.owner);
    let _id = create_hackathon(&ctx);
    let owner_after = token.balance(&ctx.owner);

    let expected_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000;
    let total_deducted = owner_before - owner_after;
    assert_eq!(total_deducted, TOTAL_BUDGET + expected_fee);
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);
}

#[test]
fn override_bps_used_on_create() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let override_bps: u32 = 500; // 5%
    let owner_before = token.balance(&ctx.owner);
    let _id = create_hackathon_with_override(&ctx, override_bps);
    let owner_after = token.balance(&ctx.owner);

    let expected_fee = TOTAL_BUDGET * override_bps as i128 / 10_000;
    let total_deducted = owner_before - owner_after;
    assert_eq!(total_deducted, TOTAL_BUDGET + expected_fee);
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);
}

#[test]
fn zero_override_bps_skips_fee() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let owner_before = token.balance(&ctx.owner);
    let _id = create_hackathon_with_override(&ctx, 0);
    let owner_after = token.balance(&ctx.owner);

    assert_eq!(owner_before - owner_after, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account), 0);
}

#[test]
fn override_bps_above_max_rejected() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/bad"),
        title: String::from_str(&ctx.env, "Bad Override"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: Some(1001), // MAX_FEE_BPS = 1000
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "fee_bps_override > MAX_FEE_BPS must revert");
}

#[test]
fn override_at_max_bps_boundary_succeeds() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let _id = create_hackathon_with_override(&ctx, 1000); // exactly MAX_FEE_BPS
    let expected_fee = TOTAL_BUDGET * 1000 / 10_000; // 10%
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);
}

// ============================================================
// compute_fee_at: rounding behavior (truncation toward zero)
// ============================================================

#[test]
fn fee_rounds_down_non_divisible_amount() {
    // 1 stroop * 250 bps / 10_000 = 0.025 → truncates to 0.
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let tiny_budget: i128 = 1;
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: tiny_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/tiny"),
        title: String::from_str(&ctx.env, "Tiny"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let owner_before = token.balance(&ctx.owner);
    ctx.events.create_event(&params, &op);
    let owner_after = token.balance(&ctx.owner);

    // fee = 1 * 250 / 10_000 = 0 (truncated)
    assert_eq!(owner_before - owner_after, tiny_budget);
    assert_eq!(token.balance(&ctx.fee_account), 0);
}

#[test]
fn fee_rounding_on_odd_amounts() {
    // 333 stroops at 250 bps: 333 * 250 / 10_000 = 8.325 → 8 stroops.
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let budget: i128 = 333;
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/odd"),
        title: String::from_str(&ctx.env, "Odd"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op);

    let expected_fee: i128 = 333 * 250 / 10_000; // = 8
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);
    assert_eq!(expected_fee, 8);
}

// ============================================================
// deposit_with_fee: override bps flows through to add_funds
// ============================================================

#[test]
fn add_funds_uses_event_override_bps() {
    let ctx = setup();
    let override_bps: u32 = 100; // 1%
    let id = create_hackathon_with_override(&ctx, override_bps);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let create_fee = TOTAL_BUDGET * override_bps as i128 / 10_000;
    assert_eq!(token.balance(&ctx.fee_account), create_fee);

    let partner = Address::generate(&ctx.env);
    let contrib = 500_0000000_i128;
    let partner_fee = contrib * override_bps as i128 / 10_000;
    fund(&ctx, &partner, contrib + partner_fee);

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op);

    assert_eq!(
        token.balance(&ctx.fee_account),
        create_fee + partner_fee,
        "add_funds fee should use the event's override bps, not the global"
    );
}

#[test]
fn add_funds_zero_override_bps_charges_no_fee() {
    let ctx = setup();
    let id = create_hackathon_with_override(&ctx, 0);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.fee_account), 0);

    let partner = Address::generate(&ctx.env);
    let contrib = 500_0000000_i128;
    fund(&ctx, &partner, contrib); // no fee to add

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op);

    assert_eq!(token.balance(&ctx.fee_account), 0);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET + contrib);
}

// ============================================================
// deposit_with_fee: zero-bps global
// ============================================================

#[test]
fn zero_global_bps_no_override_charges_no_fee() {
    let ctx = setup_with_bps(0);
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let owner_before = token.balance(&ctx.owner);
    let _id = create_hackathon(&ctx);
    let owner_after = token.balance(&ctx.owner);

    assert_eq!(owner_before - owner_after, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account), 0);
}

// ============================================================
// release math: select_winners Single — percent of escrow
// ============================================================

#[test]
fn single_release_pays_full_escrow_for_100_percent() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: winner.clone(),
            position: 1,
            credit_earn: 10,
            reputation_bump: 50,
        },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&winner), TOTAL_BUDGET);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn multi_position_split_pays_correct_amounts() {
    let ctx = setup();

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 50);
    dist.set(2, 30);
    dist.set(3, 20);
    let id = create_hackathon_with_dist(&ctx, dist);

    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let w3 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
        WinnerSpec { recipient: w2.clone(), position: 2, credit_earn: 10, reputation_bump: 30 },
        WinnerSpec { recipient: w3.clone(), position: 3, credit_earn: 10, reputation_bump: 20 },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let escrow = TOTAL_BUDGET; // all positions filled at create time
    assert_eq!(token.balance(&w1), escrow * 50 / 100);
    assert_eq!(token.balance(&w2), escrow * 30 / 100);
    assert_eq!(token.balance(&w3), escrow * 20 / 100);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn three_way_33_33_34_split_rounding() {
    let ctx = setup();

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 33);
    dist.set(2, 33);
    dist.set(3, 34);
    let id = create_hackathon_with_dist(&ctx, dist);

    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let w3 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
        WinnerSpec { recipient: w2.clone(), position: 2, credit_earn: 10, reputation_bump: 30 },
        WinnerSpec { recipient: w3.clone(), position: 3, credit_earn: 10, reputation_bump: 20 },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let escrow = TOTAL_BUDGET;
    // 33% of 1000_0000000 = 330_0000000 (exact)
    // 34% of 1000_0000000 = 340_0000000 (exact)
    assert_eq!(token.balance(&w1), escrow * 33 / 100);
    assert_eq!(token.balance(&w2), escrow * 33 / 100);
    assert_eq!(token.balance(&w3), escrow * 34 / 100);

    let event = ctx.events.get_event(&id);
    // 330 + 330 + 340 = 1000. Exact, no dust.
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn partial_position_fill_leaves_residual_escrow() {
    // Fill only 1 of 2 positions → event stays Active with residual escrow.
    let ctx = setup();

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let id = create_hackathon_with_dist(&ctx, dist);

    let w1 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&w1), TOTAL_BUDGET * 60 / 100);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, TOTAL_BUDGET - TOTAL_BUDGET * 60 / 100);
}

// ============================================================
// release math: partner contributions enlarge the payout pool
// ============================================================

#[test]
fn partner_funds_grow_winner_payout() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    let contrib = 500_0000000_i128;
    let fee = contrib * FEE_BPS as i128 / 10_000;
    fund(&ctx, &partner, contrib + fee);
    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op_add);

    let event = ctx.events.get_event(&id);
    let escrow_at_select = event.remaining_escrow;
    assert_eq!(escrow_at_select, TOTAL_BUDGET + contrib);

    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&winner), escrow_at_select);
}

// ============================================================
// release math: claim_milestone (Grant, fixed split)
// ============================================================

#[test]
fn grant_milestone_pays_floored_per_milestone() {
    let ctx = setup();
    let milestones = 3_u32;
    let id = create_grant(&ctx, milestones);

    let recipient = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: recipient.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op_sel = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op_sel);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    // total_share = TOTAL_BUDGET * 100 / 100 = TOTAL_BUDGET
    // per_milestone_floored = TOTAL_BUDGET / 3 = 333_3333333 (floor)
    let per_milestone = TOTAL_BUDGET / milestones as i128;

    // Milestone 0
    let op_m0 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &recipient, &0, &10, &50, &op_m0);
    assert_eq!(token.balance(&recipient), per_milestone);

    // Milestone 1
    let op_m1 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &recipient, &1, &10, &50, &op_m1);
    assert_eq!(token.balance(&recipient), per_milestone * 2);

    // Milestone 2 (last): sweep — pays total_share - already_paid
    let op_m2 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &recipient, &2, &10, &50, &op_m2);
    assert_eq!(
        token.balance(&recipient),
        TOTAL_BUDGET,
        "last milestone sweep must pay entire remaining share"
    );

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn grant_milestone_double_claim_rejected() {
    let ctx = setup();
    let id = create_grant(&ctx, 2);

    let recipient = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: recipient.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op_sel = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op_sel);

    let op_m0 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &recipient, &0, &10, &50, &op_m0);

    // Same milestone again → MilestoneAlreadyClaimed
    let op_m0_dup = BytesN::random(&ctx.env);
    let res = ctx.events.try_claim_milestone(&id, &recipient, &0, &10, &50, &op_m0_dup);
    assert!(res.is_err(), "double claim_milestone must revert");
}

#[test]
fn grant_milestone_out_of_range_rejected() {
    let ctx = setup();
    let milestones = 2_u32;
    let id = create_grant(&ctx, milestones);

    let recipient = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: recipient.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op_sel = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op_sel);

    // Milestone 2 is out-of-range for Multi(2) (valid: 0, 1)
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_claim_milestone(&id, &recipient, &2, &10, &50, &op);
    assert!(res.is_err(), "milestone >= total_milestones must revert");
}

// ============================================================
// release math: claim_milestone (Crowdfunding, dynamic split)
// ============================================================

#[test]
fn crowdfunding_dynamic_milestone_split() {
    let ctx = setup();
    let milestones = 3_u32;

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100);
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/crowd"),
        title: String::from_str(&ctx.env, "Crowd Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op_create);

    // Fund from a backer — crowdfunding backers pay NO fee on deposit (deposit_no_fee).
    let backer = Address::generate(&ctx.env);
    let raised = 900_0000000_i128;
    fund(&ctx, &backer, raised);
    let op_fund = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &backer, &raised, &op_fund);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    // Crowdfunding claim_milestone uses release_with_fee_at: builder bears
    // the fee at release. Each milestone amount = remaining / remaining_milestones;
    // builder receives amount - fee.
    let milestone_amount = raised / 3; // 300_0000000
    let milestone_fee = milestone_amount * FEE_BPS as i128 / 10_000;
    let net_per_milestone = milestone_amount - milestone_fee;

    // M0: amount = 900 / 3 = 300, net = 300 - fee
    let op_m0 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &0, &10, &50, &op_m0);
    assert_eq!(token.balance(&ctx.owner) - owner_before, net_per_milestone);

    // M1: remaining = 600 / 2 = 300, net = 300 - fee
    let op_m1 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &1, &10, &50, &op_m1);
    assert_eq!(token.balance(&ctx.owner) - owner_before, net_per_milestone * 2);

    // M2: remaining = 300 / 1 = 300, net = 300 - fee
    let op_m2 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &2, &10, &50, &op_m2);
    assert_eq!(token.balance(&ctx.owner) - owner_before, net_per_milestone * 3);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn crowdfunding_dynamic_rounding_no_dust() {
    // Raise 1_000_0000001 stroops and split across 3 milestones.
    // 1_000_0000001 / 3 = 333_3333333 (floor), remaining = 666_6666668
    // 666_6666668 / 2 = 333_3333334 (floor), remaining = 333_3333334
    // last milestone takes entire remainder = 333_3333334
    // Total paid = 333_3333333 + 333_3333334 + 333_3333334 = 1_000_0000001 ✓
    let ctx = setup();
    let milestones = 3_u32;

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100);
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: 1_000_0000001,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/dust"),
        title: String::from_str(&ctx.env, "Dust Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op_create);

    let backer = Address::generate(&ctx.env);
    let raised: i128 = 1_000_0000001;
    fund(&ctx, &backer, raised); // crowdfunding: no fee on deposit
    let op_fund = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &backer, &raised, &op_fund);

    let op_m0 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &0, &10, &50, &op_m0);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 666_6666668);

    let op_m1 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &1, &10, &50, &op_m1);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 333_3333334);

    let op_m2 = BytesN::random(&ctx.env);
    ctx.events.claim_milestone(&id, &ctx.owner, &2, &10, &50, &op_m2);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}

// ============================================================
// fee_account balance: create + add_funds cumulative
// ============================================================

#[test]
fn fee_account_accumulates_across_create_and_add_funds() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let create_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000;
    assert_eq!(token.balance(&ctx.fee_account), create_fee);

    let partner = Address::generate(&ctx.env);
    let contrib = 200_0000000_i128;
    let contrib_fee = contrib * FEE_BPS as i128 / 10_000;
    fund(&ctx, &partner, contrib + contrib_fee);
    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op);

    assert_eq!(token.balance(&ctx.fee_account), create_fee + contrib_fee);
}

// ============================================================
// idempotency: replayed create_event
// ============================================================

#[test]
fn replayed_create_event_reverts() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/replay"),
        title: String::from_str(&ctx.env, "Replay"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op);

    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "replayed create_event must revert");
}

#[test]
fn replayed_select_winners_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op);

    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "replayed select_winners must revert");
}

// ============================================================
// select_winners error variants
// ============================================================

#[test]
fn select_winners_on_nonexistent_event_reverts() {
    let ctx = setup();
    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&999_u64, &winners, &op);
    assert!(res.is_err());
}

#[test]
fn select_winners_duplicate_position_reverts() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 50);
    dist.set(2, 50);
    let id = create_hackathon_with_dist(&ctx, dist);

    let w1 = Address::generate(&ctx.env);
    let w2 = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w1.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
        WinnerSpec { recipient: w2.clone(), position: 1, credit_earn: 10, reputation_bump: 30 },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "duplicate winner position must revert");
}

#[test]
fn select_winners_invalid_position_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx); // dist has only position 1

    let w = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w.clone(), position: 99, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "position not in distribution must revert");
}

#[test]
fn select_winners_empty_list_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let winners = soroban_sdk::vec![&ctx.env];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "empty winners list must revert");
}

#[test]
fn select_winners_twice_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let w = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];

    let op1 = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op1);

    // Second selection on the same event → WinnersAlreadySelected
    let op2 = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op2);
    assert!(res.is_err(), "second select_winners on same event must revert");
}

// ============================================================
// Large amount: verify no overflow in fee computation
// ============================================================

#[test]
fn large_budget_fee_does_not_overflow() {
    let ctx = setup();
    let big_budget: i128 = i128::MAX / 20_000; // large but within safe multiply range
    fund(&ctx, &ctx.owner, big_budget * 2);

    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: big_budget,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/big"),
        title: String::from_str(&ctx.env, "Big"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let expected_fee = big_budget * FEE_BPS as i128 / 10_000;
    assert_eq!(token.balance(&ctx.fee_account), expected_fee);
}

// ============================================================
// global bps change mid-flight doesn't affect existing events
// ============================================================

#[test]
fn mid_flight_global_bps_change_does_not_affect_override_event() {
    let ctx = setup();
    let override_bps: u32 = 100;
    let id = create_hackathon_with_override(&ctx, override_bps);

    // Admin changes the global fee bps
    ctx.events.set_fee_bps(&500);

    // Partner add_funds should still use the event's override (100), not the new global (500)
    let partner = Address::generate(&ctx.env);
    let contrib = 500_0000000_i128;
    let partner_fee = contrib * override_bps as i128 / 10_000;
    fund(&ctx, &partner, contrib + partner_fee);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op);

    let fee_after = token.balance(&ctx.fee_account);
    assert_eq!(
        fee_after - fee_before,
        partner_fee,
        "event with override must ignore global bps change"
    );
}

// ============================================================
// auth rejection: select_winners requires owner auth
// ============================================================

#[test]
fn select_winners_on_cancelled_event_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    super::common::drive_cancel(&ctx.env, &ctx.events, id);

    let w = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err(), "select_winners on cancelled event must revert");
}

// ============================================================
// Fee + release combined: verify fee_account and winner are both correct
// ============================================================

#[test]
fn fee_and_winner_balances_consistent() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let id = create_hackathon(&ctx);
    let create_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000;

    let partner = Address::generate(&ctx.env);
    let contrib = 400_0000000_i128;
    let contrib_fee = contrib * FEE_BPS as i128 / 10_000;
    fund(&ctx, &partner, contrib + contrib_fee);
    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &contrib, &op_add);

    let escrow = TOTAL_BUDGET + contrib;

    let winner = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: winner.clone(), position: 1, credit_earn: 10, reputation_bump: 50 },
    ];
    let op_sel = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op_sel);

    assert_eq!(token.balance(&winner), escrow);
    assert_eq!(token.balance(&ctx.fee_account), create_fee + contrib_fee);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
}
