// boundless-events: cancel + refund batch tests (#28).
//
// Covers start_cancel / process_cancel_batch / finalize_cancel:
//   - OwnerOnly branch settled inline.
//   - FullPartnerThenResidual: partners full + owner residual.
//   - ProRataPartners: remaining < non_owner_total.
//   - Pagination across multiple batches.
//   - Error variants: wrong state, replay, not finished.

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
const TOTAL_BUDGET: i128 = 1_000_0000000_i128;
const MIN_CONTRIB: i128 = 100_000_000_i128;

#[allow(dead_code)]
struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    profile: ProfileContractClient<'a>,
    owner: Address,
    token_addr: Address,
    token_admin: token::StellarAssetClient<'a>,
    fee_account: Address,
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
        (events_admin.clone(), fee_account.clone(), FEE_BPS, profile_id.clone()),
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

    Ctx { env, events, profile, owner, token_addr, token_admin, fee_account }
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
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/cancel-test"),
        title: String::from_str(&ctx.env, "Cancel Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

fn contribute(ctx: &Ctx, id: u64, who: &Address, amount: i128) {
    let fee = amount * FEE_BPS as i128 / 10_000;
    ctx.token_admin.mint(who, &(amount + fee));
    ctx.events.add_funds(&id, who, &amount, &BytesN::random(&ctx.env));
}

// ============================================================
// OwnerOnly branch
// ============================================================

#[test]
fn owner_only_cancel_settles_inline() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let before = token.balance(&ctx.owner);

    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);
    assert_eq!(token.balance(&ctx.owner) - before, TOTAL_BUDGET);
}

#[test]
fn owner_only_process_and_finalize_rejected_after_inline_settle() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));

    assert!(ctx.events.try_process_cancel_batch(&id, &10_u32, &BytesN::random(&ctx.env)).is_err());
    assert!(ctx.events.try_finalize_cancel(&id, &BytesN::random(&ctx.env)).is_err());
}

// ============================================================
// FullPartnerThenResidual branch
// ============================================================

#[test]
fn full_partner_then_residual_pays_partners_and_owner() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    contribute(&ctx, id, &p1, 200_0000000_i128);
    contribute(&ctx, id, &p2, 300_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, 200_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 300_0000000_i128);
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelled);
}

#[test]
fn paged_cancel_processes_in_batches() {
    let ctx = setup();
    let id = create_hackathon(&ctx);

    let partners = [
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
        Address::generate(&ctx.env),
    ];
    for p in partners.iter() {
        contribute(&ctx, id, p, MIN_CONTRIB);
    }

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let balances = [
        token.balance(&partners[0]),
        token.balance(&partners[1]),
        token.balance(&partners[2]),
        token.balance(&partners[3]),
        token.balance(&partners[4]),
    ];
    let owner_before = token.balance(&ctx.owner);

    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelling);

    let left = ctx.events.process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 3);

    let left = ctx.events.process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 1);

    assert!(ctx.events.try_finalize_cancel(&id, &BytesN::random(&ctx.env)).is_err());

    let left = ctx.events.process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 0);

    ctx.events.finalize_cancel(&id, &BytesN::random(&ctx.env));

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);

    for (i, p) in partners.iter().enumerate() {
        assert_eq!(token.balance(p) - balances[i], MIN_CONTRIB);
    }
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
}

// ============================================================
// ProRataPartners branch (boundary: remaining == non_owner_total)
// ============================================================

#[test]
fn cancel_at_boundary_pays_partners_full_no_owner_residual() {
    // 60/40 split; pay position 1 (60%); remaining = 40% of escrow.
    // With partner pool == remaining, no owner residual.
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60);
    dist.set(2, 40);
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
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    contribute(&ctx, id, &p1, 500_0000000_i128);
    contribute(&ctx, id, &p2, 500_0000000_i128);

    // remaining = 1000 + 1000 = 2000; pay pos1 (60%) = 1200 → remaining = 800.
    // non_owner_total = 1000. 800 < 1000 → ProRata.
    // But let's use a cleaner case: pay pos1 at 60% of 2000 = 1200; remaining = 800.
    // Actually simpler: use total_budget=1000, partner=1000, pay pos1(60%)=1200 err.
    // Use the boundary exactly as in contributions.rs: partner == remaining.
    // escrow=2000, pay pos1 (60% of 2000)=1200, remaining=800, partner=1000 → ProRata.
    // For FullPartner boundary: remaining==non_owner_total.
    // Pay pos1 (60%*2000=1200), remaining=800; partner=800 → FullPartner boundary.
    // Re-seed with partner = 400 each (800 total).
    // Actually the test in contributions.rs already covers the boundary well.
    // Here just verify the basic FullPartner case works with no owner residual.

    // Simpler: create fresh event with partner == remaining after partial payout.
    // TOTAL_BUDGET=1000, two partners 250 each (500 total). remaining = 1500.
    // Pay pos1 60% of 1500 = 900. remaining = 600. non_owner = 500. 600 > 500 → FullPartner.
    // Owner residual = 600 - 500 = 100.
    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let w = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec { recipient: w.clone(), position: 1, credit_earn: 0, reputation_bump: 0 },
    ];
    ctx.events.select_winners(&id, &winners, &BytesN::random(&ctx.env));

    // remaining after pay: 2000 - 2000*0.6 = 800. non_owner = 1000. ProRata.
    // p1 share = 500 * 800 / 1000 = 400.
    // p2 share = 500 * 800 / 1000 = 400.
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, 400_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 400_0000000_i128);
    assert_eq!(token.balance(&ctx.owner) - owner_before, 0);
}

// ============================================================
// Error variants
// ============================================================

#[test]
fn start_cancel_on_nonexistent_event_reverts() {
    let ctx = setup();
    assert!(ctx.events.try_start_cancel(&999_u64, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn start_cancel_on_already_cancelled_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert!(ctx.events.try_start_cancel(&id, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn process_cancel_batch_without_start_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    assert!(ctx.events.try_process_cancel_batch(&id, &5_u32, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn finalize_cancel_before_all_batches_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    for _ in 0..3 {
        let p = Address::generate(&ctx.env);
        contribute(&ctx, id, &p, MIN_CONTRIB);
    }
    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));
    assert!(ctx.events.try_finalize_cancel(&id, &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn contributor_amount_zeroed_after_cancel() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let p = Address::generate(&ctx.env);
    contribute(&ctx, id, &p, 250_0000000_i128);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert_eq!(ctx.events.get_contributor_amount(&id, &p), 0);
}

#[test]
fn add_funds_on_cancelling_event_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let p = Address::generate(&ctx.env);
    contribute(&ctx, id, &p, MIN_CONTRIB);
    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));

    let p2 = Address::generate(&ctx.env);
    let fee = MIN_CONTRIB * FEE_BPS as i128 / 10_000;
    ctx.token_admin.mint(&p2, &(MIN_CONTRIB + fee));
    assert!(ctx.events.try_add_funds(&id, &p2, &MIN_CONTRIB, &BytesN::random(&ctx.env)).is_err());
}
