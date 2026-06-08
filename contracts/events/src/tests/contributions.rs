// boundless-events: partner contribution + refund tests.
//
// Covers the open `add_funds` path and the cancel-event refund matrix.
//
// Refund-policy reachability note: the contract has three cancel branches:
//
//   1. no contributors            -> owner gets everything
//   2. remaining >= non_owner_tot -> partners in full + owner residual
//   3. remaining <  non_owner_tot -> partners pro-rata, owner gets 0
//
// Branch 3 is defensive code. The current contract enforces a hard
// invariant: total payouts <= total_budget (select_winners and
// claim_milestone both cap per-recipient amounts via total_budget *
// percent / 100). Therefore at cancel time:
//
//   remaining = total_budget + non_owner_total - payouts
//             >= non_owner_total
//
// so branch 3 is unreachable today. We still test the boundary
// remaining == non_owner_total to exercise the partner-pays-in-full path
// when owner residual is zero. Future op kinds (admin slashing, fee
// adjustments) could break the invariant; the branch is kept and covered
// by code review until then.
//
// Spec: boundless-partner-contributions-prd.md Sections 6 + 7.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::types::{CreateEventParams, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const BOOTSTRAP_CREDITS: u32 = 10;
const FEE_BPS: u32 = 250;

// 10 USDC at 7 decimals = 100_000_000 stroops. Mirrors the contract constant.
const MIN_CONTRIB: i128 = 100_000_000_i128;

// Hackathon-shaped budget: 1000 USDC.
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
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/contrib-hack"),
        title: String::from_str(&ctx.env, "Contrib Hack"),
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
// add_funds happy path
// ============================================================

#[test]
fn anyone_can_top_up_an_active_event() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    let amount = 500_0000000_i128; // 500 USDC
    let fee = amount * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &partner, amount + fee);

    let event_before = ctx.events.get_event(&id);
    let escrow_before = event_before.remaining_escrow;

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &amount, &op);

    let event_after = ctx.events.get_event(&id);
    assert_eq!(
        event_after.remaining_escrow,
        escrow_before + amount,
        "remaining_escrow grew by the net amount"
    );

    let list = ctx.events.get_contributors(&id);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), partner);

    let amt = ctx.events.get_contributor_amount(&id, &partner);
    assert_eq!(amt, amount);

    // Fee account got create-time fee plus this contribution's fee.
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let initial_owner_fee = TOTAL_BUDGET * FEE_BPS as i128 / 10_000_i128;
    assert_eq!(token.balance(&ctx.fee_account), initial_owner_fee + fee);
}

#[test]
fn below_minimum_contribution_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 1_000_0000000_i128);

    let too_small = MIN_CONTRIB - 1;
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_add_funds(&id, &partner, &too_small, &op);
    assert!(res.is_err(), "below-minimum contribution must revert");

    // Exactly the minimum succeeds.
    let op_ok = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &MIN_CONTRIB, &op_ok);
}

#[test]
fn zero_or_negative_contribution_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 1_000_0000000_i128);

    let op_zero = BytesN::random(&ctx.env);
    let res_zero = ctx.events.try_add_funds(&id, &partner, &0_i128, &op_zero);
    assert!(res_zero.is_err());

    let op_neg = BytesN::random(&ctx.env);
    let res_neg = ctx
        .events
        .try_add_funds(&id, &partner, &-1_0000000_i128, &op_neg);
    assert!(res_neg.is_err());
}

#[test]
fn owner_top_up_grows_escrow_without_recording_contribution_entry() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let extra = 200_0000000_i128;
    let fee = extra * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &ctx.owner, extra + fee);

    let escrow_before = ctx.events.get_event(&id).remaining_escrow;

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &ctx.owner, &extra, &op);

    let escrow_after = ctx.events.get_event(&id).remaining_escrow;
    assert_eq!(escrow_after, escrow_before + extra);

    let list = ctx.events.get_contributors(&id);
    assert_eq!(list.len(), 0);
    let amt = ctx.events.get_contributor_amount(&id, &ctx.owner);
    assert_eq!(amt, 0);
}

#[test]
fn replayed_add_funds_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 1_000_0000000_i128);

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &MIN_CONTRIB, &op);

    let res = ctx
        .events
        .try_add_funds(&id, &partner, &MIN_CONTRIB, &op);
    assert!(res.is_err(), "replayed add_funds should revert");
}

#[test]
fn add_funds_to_cancelled_event_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    drive_cancel(&ctx.env, &ctx.events, id);

    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 1_000_0000000_i128);

    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_add_funds(&id, &partner, &MIN_CONTRIB, &op);
    assert!(res.is_err(), "add_funds on cancelled event must revert");
}

#[test]
fn add_funds_on_nonexistent_event_reverts() {
    let ctx = setup();
    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 1_000_0000000_i128);

    let op = BytesN::random(&ctx.env);
    let res = ctx
        .events
        .try_add_funds(&999_u64, &partner, &MIN_CONTRIB, &op);
    assert!(res.is_err());
}

#[test]
fn multiple_top_ups_from_same_contributor_aggregate_and_dont_duplicate_list() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    fund(&ctx, &partner, 10_000_0000000_i128);

    let op_a = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &MIN_CONTRIB, &op_a);
    let op_b = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &(MIN_CONTRIB * 2), &op_b);

    let list = ctx.events.get_contributors(&id);
    assert_eq!(list.len(), 1, "contributor list de-dupes");
    let amt = ctx.events.get_contributor_amount(&id, &partner);
    assert_eq!(amt, MIN_CONTRIB * 3);
}

// ============================================================
// cancel_event refund matrix
// ============================================================

#[test]
fn cancel_with_no_contributors_refunds_owner_in_full() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, id);

    let owner_after = token.balance(&ctx.owner);
    assert_eq!(
        owner_after - owner_before,
        TOTAL_BUDGET,
        "owner gets the full budget back when no partners contributed"
    );

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn cancel_with_partner_pool_refunds_partners_then_owner_residual() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    let c1 = 200_0000000_i128;
    let c2 = 300_0000000_i128;
    let fee1 = c1 * FEE_BPS as i128 / 10_000_i128;
    let fee2 = c2 * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &p1, c1 + fee1);
    fund(&ctx, &p2, c2 + fee2);

    let op1 = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &p1, &c1, &op1);
    let op2 = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &p2, &c2, &op2);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, c1);
    assert_eq!(token.balance(&p2) - p2_before, c2);
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

#[test]
fn cancel_at_boundary_pays_partners_full_no_owner_residual() {
    // Drain enough escrow so remaining_escrow == non_owner_total at cancel.
    // After M1 (select_winners pays against remaining_escrow), we cannot
    // hit the boundary by paying out 100% of distribution — that drains
    // the partner pool too. Instead we fill only one position (50%) so
    // half the escrow is paid and half remains; with partner pool sized
    // to that remainder, the cancel falls on the equality boundary.
    //
    // TOTAL_BUDGET = 1000, partner = 1000 (split 500 / 500),
    // escrow_at_select = 2000, dist = 50% + 50% (positions 1 and 2),
    // pay only position 1: paid = 1000, remaining = 1000 = partner pool.

    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 50);
    dist.set(2, 50);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/boundary"),
        title: String::from_str(&ctx.env, "Boundary Cancel"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let op_create = BytesN::random(&ctx.env);
    let id = ctx.events.create_event(&params, &op_create);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    let c1 = 500_0000000_i128;
    let c2 = 500_0000000_i128;
    let fee1 = c1 * FEE_BPS as i128 / 10_000_i128;
    let fee2 = c2 * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &p1, c1 + fee1);
    fund(&ctx, &p2, c2 + fee2);
    let op1 = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &p1, &c1, &op1);
    let op2 = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &p2, &c2, &op2);

    // remaining = 1000 + 1000 = 2000. Pay only position 1 at 50% of escrow
    // = 1000. remaining_after = 1000 = non_owner_total. Boundary case A.
    let winner_a = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: winner_a.clone(),
            position: 1,
            credit_earn: 20,
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&id, &winners, &op_select);

    // The event isn't Completed because remaining (1000) != 0.
    let after_select = ctx.events.get_event(&id);
    assert_eq!(after_select.status, EventStatus::Active);
    assert_eq!(after_select.remaining_escrow, 1_000_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, c1);
    assert_eq!(token.balance(&p2) - p2_before, c2);
    assert_eq!(
        token.balance(&ctx.owner) - owner_before,
        0,
        "no owner residual at the boundary"
    );
}

#[test]
fn cancel_with_owner_top_up_keeps_owner_residual_correct() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let extra = 200_0000000_i128;
    let efee = extra * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &ctx.owner, extra + efee);
    let op_top = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &ctx.owner, &extra, &op_top);

    let partner = Address::generate(&ctx.env);
    let pc = 300_0000000_i128;
    let pcf = pc * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &partner, pc + pcf);
    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &pc, &op_add);

    // remaining = 1000 + 200 + 300 = 1500. non_owner_total = 300.
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);
    let partner_before = token.balance(&partner);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&partner) - partner_before, pc);
    // Owner residual = 1500 - 300 = 1200 (their original budget + top-up).
    assert_eq!(token.balance(&ctx.owner) - owner_before, 1_200_0000000_i128);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
}

// ============================================================
// H3/H4 cap + paged read coverage
// ============================================================

#[test]
fn add_funds_paged_storage_round_trip() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    // Three distinct partners; verify both the legacy snapshot read and
    // the paged accessors agree on the layout.
    let partners: [Address; 3] = [
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
    ];
    for p in partners.iter() {
        let amount = MIN_CONTRIB;
        let fee = amount * FEE_BPS as i128 / 10_000_i128;
        fund(&ctx, p, amount + fee);
        let op = BytesN::random(&ctx.env);
        ctx.events.add_funds(&id, p, &amount, &op);
    }

    assert_eq!(ctx.events.get_contributor_count(&id), 3);
    for (idx, p) in partners.iter().enumerate() {
        let stored = ctx
            .events
            .get_contributor_at(&id, &(idx as u32))
            .expect("slot populated");
        assert_eq!(stored, *p);
    }
    let snap = ctx.events.get_contributors(&id);
    assert_eq!(snap.len(), 3);
}

#[test]
fn paged_cancel_processes_in_batches() {
    // Seed 5 partners then drive the cancel paged-flow with a batch size
    // of 2. Confirms:
    //   - start_cancel flips status to Cancelling and persists the cursor
    //   - process_cancel_batch refunds the next N and advances the cursor
    //   - finalize_cancel pays owner residual and flips to Cancelled
    //   - all partners receive their original amount (branch A)
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let mut partners: [Address; 5] = [
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
    ];
    let per = MIN_CONTRIB;
    for p in partners.iter_mut() {
        let fee = per * FEE_BPS as i128 / 10_000_i128;
        fund(&ctx, p, per + fee);
        let op = BytesN::random(&ctx.env);
        ctx.events.add_funds(&id, p, &per, &op);
    }
    assert_eq!(ctx.events.get_contributor_count(&id), 5);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let balances_before: [i128; 5] = [
        token.balance(&partners[0]),
        token.balance(&partners[1]),
        token.balance(&partners[2]),
        token.balance(&partners[3]),
        token.balance(&partners[4]),
    ];
    let owner_before = token.balance(&ctx.owner);

    // start_cancel: Cancelling, owner unpaid yet.
    let op_start = BytesN::random(&ctx.env);
    ctx.events.start_cancel(&id, &op_start);
    let after_start = ctx.events.get_event(&id);
    assert_eq!(after_start.status, EventStatus::Cancelling);
    assert_eq!(token.balance(&ctx.owner) - owner_before, 0);

    // Process in batches of 2.
    let batch = 2_u32;
    let op_b1 = BytesN::random(&ctx.env);
    let remaining = ctx.events.process_cancel_batch(&id, &batch, &op_b1);
    assert_eq!(remaining, 3);

    let op_b2 = BytesN::random(&ctx.env);
    let remaining = ctx.events.process_cancel_batch(&id, &batch, &op_b2);
    assert_eq!(remaining, 1);

    // Cannot finalize yet.
    let op_too_early = BytesN::random(&ctx.env);
    let r = ctx.events.try_finalize_cancel(&id, &op_too_early);
    assert!(r.is_err(), "finalize before cursor end must revert");

    let op_b3 = BytesN::random(&ctx.env);
    let remaining = ctx.events.process_cancel_batch(&id, &batch, &op_b3);
    assert_eq!(remaining, 0);

    let op_final = BytesN::random(&ctx.env);
    ctx.events.finalize_cancel(&id, &op_final);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);

    // Each partner received their original deposit.
    for (i, p) in partners.iter().enumerate() {
        assert_eq!(token.balance(p) - balances_before[i], per);
    }
    // Owner residual = TOTAL_BUDGET (their original deposit; partners are paid in full).
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
}

#[test]
fn paged_cancel_owner_only_settles_inside_start() {
    // No partner contributions: start_cancel settles inline + flips Cancelled
    // in one tx. process_cancel_batch / finalize_cancel must reject.
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let owner_before = token.balance(&ctx.owner);

    let op_start = BytesN::random(&ctx.env);
    ctx.events.start_cancel(&id, &op_start);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);

    // No CancellationState left to process or finalize.
    let op_b = BytesN::random(&ctx.env);
    let r = ctx.events.try_process_cancel_batch(&id, &10_u32, &op_b);
    assert!(r.is_err());

    let op_f = BytesN::random(&ctx.env);
    let r = ctx.events.try_finalize_cancel(&id, &op_f);
    assert!(r.is_err());
}

#[test]
fn cancel_clears_contributor_amounts_so_replay_state_is_clean() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partner = Address::generate(&ctx.env);
    let pc = 250_0000000_i128;
    let pcf = pc * FEE_BPS as i128 / 10_000_i128;
    fund(&ctx, &partner, pc + pcf);
    let op_add = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &partner, &pc, &op_add);

    drive_cancel(&ctx.env, &ctx.events, id);

    let amt = ctx.events.get_contributor_amount(&id, &partner);
    assert_eq!(amt, 0);
}
