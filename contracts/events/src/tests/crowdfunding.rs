// boundless-events: crowdfunding tests.
//
// Covers the Pillar::Crowdfunding paths:
//   - create_event with zero owner deposit, builder auto-registered as Winner
//   - validate_create rejects single-release / missing-deadline / wrong dist
//   - add_funds drives escrow from 0 up to whatever was raised
//   - claim_milestone dynamic math: amount = remaining / (total - claimed)
//   - last milestone drains remainder including rounding dust
//   - select_winners and submit are rejected for Crowdfunding
//   - cancel_event refunds all contributors (no owner deposit to refund)
//
// Spec: boundless-crowdfunding-prd.md.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String, Vec as SorobanVec,
};

use super::common::drive_cancel;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const FEE_BPS: u32 = 250;

// Builder's stated funding goal (informational only on-chain).
const FUNDING_GOAL: i128 = 1_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    #[allow(dead_code)]
    profile: ProfileContractClient<'a>,
    builder: Address,
    events_admin: Address,
    fee_account: Address,
    token_addr: Address,
    token_admin: token::StellarAssetClient<'a>,
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

    // Builder pays nothing upfront, so no mint needed for them.
    let builder = Address::generate(&env);

    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        profile,
        builder,
        events_admin,
        fee_account,
        token_addr,
        token_admin,
    }
}

fn single_dist_100_at_1(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_campaign(ctx: &Ctx, milestones: u32) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.builder.clone(),
        token: ctx.token_addr.clone(),
        total_budget: FUNDING_GOAL,
        release_kind: ReleaseKind::Multi(milestones),
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/cf/1"),
        title: String::from_str(&ctx.env, "Open-Source Crawler"),
        deadline: Some(ctx.env.ledger().timestamp() + 30 * 86_400),
        winner_distribution: single_dist_100_at_1(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    ctx.events.create_event(&params, &op)
}

fn fund(ctx: &Ctx, addr: &Address, amount: i128) {
    ctx.token_admin.mint(addr, &amount);
}

fn back(ctx: &Ctx, id: u64, who: &Address, amount: i128) {
    let fee = amount * FEE_BPS as i128 / 10_000_i128;
    fund(ctx, who, amount + fee);
    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, who, &amount, &op);
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn create_with_zero_owner_deposit_and_auto_registered_winner() {
    let ctx = setup();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let builder_before = token.balance(&ctx.builder);

    let id = create_campaign(&ctx, 3);

    // No deposit pulled from builder. Builder balance unchanged.
    assert_eq!(token.balance(&ctx.builder), builder_before);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Crowdfunding);
    assert_eq!(event.total_budget, FUNDING_GOAL);
    assert_eq!(
        event.remaining_escrow, 0,
        "crowdfunding starts with empty escrow"
    );

    // Builder auto-registered as Winner at position 1, milestone=None.
    let winners = ctx.events.get_winners(&id);
    assert_eq!(winners.len(), 1);
    let w = winners.get(0).unwrap();
    assert_eq!(w.recipient, ctx.builder);
    assert_eq!(w.position, 1);
    assert!(w.milestone.is_none());
    assert!(w.paid_at.is_none());
}

#[test]
fn create_rejects_single_release_kind() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.builder.clone(),
        token: ctx.token_addr.clone(),
        total_budget: FUNDING_GOAL,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad CF"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist_100_at_1(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "single release must be rejected");
}

#[test]
fn create_rejects_missing_deadline() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.builder.clone(),
        token: ctx.token_addr.clone(),
        total_budget: FUNDING_GOAL,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad CF"),
        deadline: None,
        winner_distribution: single_dist_100_at_1(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err());
}

#[test]
fn create_rejects_distribution_with_multiple_positions() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.builder.clone(),
        token: ctx.token_addr.clone(),
        total_budget: FUNDING_GOAL,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad CF"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err());
}

// ============================================================
// add_funds
// ============================================================

#[test]
fn community_top_ups_raise_escrow_from_zero() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);

    back(&ctx, id, &p1, 200_0000000_i128);
    back(&ctx, id, &p2, 300_0000000_i128);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 500_0000000_i128);
    let list = ctx.events.get_contributors(&id);
    assert_eq!(list.len(), 2);
}

#[test]
fn builder_top_up_does_not_appear_in_contributor_list() {
    // The builder is allowed to add funds (matching the owner-top-up rule).
    // Their entry shouldn't appear in ContributorList so cancel_event treats
    // them as residual, not partner.
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let extra = 100_0000000_i128;
    let fee = extra * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &ctx.builder, extra + fee);
    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &ctx.builder, &extra, &op);

    let list = ctx.events.get_contributors(&id);
    assert_eq!(list.len(), 0);
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, extra);
}

// ============================================================
// claim_milestone dynamic math
// ============================================================

#[test]
fn claim_milestone_splits_evenly_and_charges_fee_at_release() {
    // 3 milestones, escrow raised = 900 USDC. The builder bears the 2.5% fee,
    // taken from each payout (backers deposited their full pledge fee-free):
    //   m0 -> 900/3 = 300 gross; builder +292.5, fee_account +7.5; leaves 600.
    //   m1 -> 600/2 = 300 gross; builder +292.5, fee_account +7.5; leaves 300.
    //   m2 -> 300 gross (final);  builder +292.5, fee_account +7.5; leaves 0.
    // Builder nets 877.5 (= 900 * 0.975); platform collects 22.5 (= 900 * .025).
    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 900_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let op_m0 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0_u32, &op_m0);
    assert_eq!(token.balance(&ctx.builder), 292_5000000_i128);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 7_5000000_i128);
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 600_0000000_i128);

    let op_m1 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &1_u32, &0_u32, &op_m1);
    assert_eq!(token.balance(&ctx.builder), 585_0000000_i128);
    assert_eq!(
        token.balance(&ctx.fee_account) - fee_before,
        15_0000000_i128
    );
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 300_0000000_i128);

    let op_m2 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &2_u32, &0_u32, &op_m2);
    assert_eq!(token.balance(&ctx.builder), 877_5000000_i128);
    assert_eq!(
        token.balance(&ctx.fee_account) - fee_before,
        22_5000000_i128
    );
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

#[test]
fn claim_milestone_last_drains_dust_with_fee() {
    // 3 milestones, raised 100_000_001 stroops (just above MIN_CONTRIB and not
    // divisible by 3). The builder nets the raised amount minus the 2.5% fee;
    // the fee account collects the fee; together they drain escrow exactly so
    // no dust is stranded, even with per-milestone rounding.
    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 100_000_001_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let builder_before = token.balance(&ctx.builder);
    let fee_before = token.balance(&ctx.fee_account);

    for m in 0u32..3 {
        let op = BytesN::random(&ctx.env);
        ctx.events.claim_milestone(&id, &ctx.builder, &m, &0, &op);
    }

    let builder_delta = token.balance(&ctx.builder) - builder_before;
    let fee_delta = token.balance(&ctx.fee_account) - fee_before;
    assert!(fee_delta > 0, "platform collects a fee at release");
    assert_eq!(
        builder_delta + fee_delta,
        100_000_001_i128,
        "builder net + fee drains all raised funds, no dust stranded"
    );
    let event = ctx.events.get_event(&id);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(event.status, EventStatus::Completed);
}

#[test]
fn claim_milestone_replay_reverts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 600_0000000_i128);

    let op = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);

    let res = ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);
    assert!(res.is_err());
}

#[test]
fn claim_milestone_out_of_range_reverts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 400_0000000_i128);

    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &2_u32, &0, &op);
    assert!(res.is_err());
}

#[test]
fn claim_milestone_with_empty_escrow_reverts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);
    assert!(res.is_err());
}

// ============================================================
// fee model: backer pays exactly their pledge, creator bears the fee
// ============================================================

#[test]
fn backer_pays_exactly_pledge_and_creator_bears_fee() {
    // Regression for the fee-on-top bug: a backer holding EXACTLY their pledge
    // must be able to fund. The old model pulled pledge + fee from the backer
    // and reverted for a wallet that held only the pledge. Now the fee is borne
    // by the builder and taken at release, so deposit pulls exactly the pledge.
    let ctx = setup();
    let id = create_campaign(&ctx, 1);

    let backer = Address::generate(&ctx.env);
    let pledge = 100_0000000_i128; // exactly 100 USDC, no slack for a fee
    fund(&ctx, &backer, pledge);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &backer, &pledge, &op);

    // Backer paid exactly their pledge; no fee taken at deposit.
    assert_eq!(token.balance(&backer), 0, "backer pays exactly the pledge");
    assert_eq!(
        token.balance(&ctx.fee_account),
        fee_before,
        "no fee charged at deposit"
    );
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, pledge);

    // Single milestone: the builder claims and the fee is taken from the payout.
    let claim = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &claim);

    let fee = pledge * FEE_BPS as i128 / 10_000_i128; // 2.5 USDC
    assert_eq!(
        token.balance(&ctx.builder),
        pledge - fee,
        "builder nets pledge minus the fee"
    );
    assert_eq!(
        token.balance(&ctx.fee_account) - fee_before,
        fee,
        "platform collects the fee at release"
    );
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, 0);
}

// ============================================================
// select_winners and submit are blocked
// ============================================================

#[test]
fn select_winners_on_crowdfunding_reverts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let spec = WinnerSpec {
        recipient: ctx.builder.clone(),
        position: 1,
        reputation_bump: 0,
    };
    let mut winners = SorobanVec::new(&ctx.env);
    winners.push_back(spec);

    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
    assert!(res.is_err());
}

#[test]
fn submit_on_crowdfunding_reverts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let op = BytesN::random(&ctx.env);
    let uri = String::from_str(&ctx.env, "ipfs://nope");
    let res = ctx.events.try_submit(&id, &ctx.builder, &uri, &op);
    assert!(res.is_err());
}

// ============================================================
// cancel_event
// ============================================================

#[test]
fn cancel_refunds_all_partners_no_owner_residual() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    back(&ctx, id, &p1, 200_0000000_i128);
    back(&ctx, id, &p2, 300_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let builder_before = token.balance(&ctx.builder);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, 200_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 300_0000000_i128);
    assert_eq!(
        token.balance(&ctx.builder) - builder_before,
        0,
        "builder put in nothing; gets nothing"
    );

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn cancel_with_no_contributions_just_marks_cancelled() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    drive_cancel(&ctx.env, &ctx.events, id);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn cancel_after_partial_claim_pro_rates_remaining() {
    // Builder claimed m0 already, then campaign is cancelled. Remaining < non_owner_total.
    // 3 milestones, raised 900, claim m0 -> 300 to builder, remaining 600.
    // Cancel: non_owner_total still 900 (we don't subtract from contributor ledger
    // on claim), remaining = 600 < 900 -> case B pro-rata.
    let ctx = setup();
    let id = create_campaign(&ctx, 3);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    back(&ctx, id, &p1, 300_0000000_i128);
    back(&ctx, id, &p2, 600_0000000_i128);

    let op_m0 = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op_m0);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);

    drive_cancel(&ctx.env, &ctx.events, id);

    // remaining = 600, non_owner_total = 900.
    // p1 share = 300 * 600 / 900 = 200.
    // p2 share = 600 * 600 / 900 = 400.
    assert_eq!(token.balance(&p1) - p1_before, 200_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 400_0000000_i128);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

// ============================================================
// M5: crowdfunding claim_milestone requires admin co-sign
// ============================================================

#[test]
fn crowdfunding_claim_milestone_requires_admin_auth() {
    // Verify the admin's address is among the required auths for a
    // crowdfunding claim. mock_all_auths_allowing_non_root_auth makes the
    // call succeed, but env.auths() records the addresses whose auth was
    // demanded, which is the audit-relevant observation.
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let p = Address::generate(&ctx.env);
    back(&ctx, id, &p, 200_0000000_i128);

    let op = BytesN::random(&ctx.env);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);

    let auths = ctx.env.auths();
    let admin_required = auths.iter().any(|(addr, _)| *addr == ctx.events_admin);
    let builder_required = auths.iter().any(|(addr, _)| *addr == ctx.builder);
    assert!(
        admin_required,
        "crowdfunding claim must demand admin co-sign"
    );
    assert!(builder_required, "builder auth still required");
}
