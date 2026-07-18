#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, EnvTestConfig},
    token, Address, BytesN, Env, Map, String,
};

use super::common::drive_cancel;
use crate::errors::Error;
use crate::storage;
use crate::types::{CreateEventParams, DataKey, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};
use boundless_profile::{ProfileContract, ProfileContractClient};

const FEE_BPS: u32 = 250;
const TOTAL_BUDGET: i128 = 1_000_0000000_i128;
const MIN_CONTRIB: i128 = 100_000_000_i128;

#[allow(dead_code)]
struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    events_id: Address,
    profile: ProfileContractClient<'a>,
    owner: Address,
    token_addr: Address,
    token_admin: token::StellarAssetClient<'a>,
    fee_account: Address,
}

fn setup<'a>() -> Ctx<'a> {
    setup_with_env(Env::default())
}

fn setup_with_env<'a>(env: Env) -> Ctx<'a> {
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
    token_admin.mint(&owner, &10_000_0000000_i128);
    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        events_id,
        profile,
        owner,
        token_addr,
        token_admin,
        fee_account,
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
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/cancel-test"),
        title: String::from_str(&ctx.env, "Cancel Test"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

fn contribute(ctx: &Ctx, id: u64, who: &Address, amount: i128) {
    let fee = amount * FEE_BPS as i128 / 10_000;
    ctx.token_admin.mint(who, &(amount + fee));
    ctx.events
        .add_funds(&id, who, &amount, &BytesN::random(&ctx.env));
}

fn remove_running_total(ctx: &Ctx, id: u64) {
    ctx.env.as_contract(&ctx.events_id, || {
        ctx.env
            .storage()
            .persistent()
            .remove(&DataKey::NonOwnerContributionTotal(id));
    });
}

fn stored_non_owner_total(ctx: &Ctx, id: u64) -> Option<i128> {
    ctx.env.as_contract(&ctx.events_id, || {
        storage::get_non_owner_contribution_total(&ctx.env, id)
    })
}

fn has_cancellation_state(ctx: &Ctx, id: u64) -> bool {
    ctx.env.as_contract(&ctx.events_id, || {
        storage::get_cancellation_state(&ctx.env, id).is_some()
    })
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

    assert!(ctx
        .events
        .try_process_cancel_batch(&id, &10_u32, &BytesN::random(&ctx.env))
        .is_err());
    assert!(ctx
        .events
        .try_finalize_cancel(&id, &BytesN::random(&ctx.env))
        .is_err());
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
    let fee_before = token.balance(&ctx.fee_account);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, 200_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 300_0000000_i128);
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
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
    let fee_before = token.balance(&ctx.fee_account);

    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelling);

    let left = ctx
        .events
        .process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 3);

    let left = ctx
        .events
        .process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 1);

    assert!(ctx
        .events
        .try_finalize_cancel(&id, &BytesN::random(&ctx.env))
        .is_err());

    let left = ctx
        .events
        .process_cancel_batch(&id, &2_u32, &BytesN::random(&ctx.env));
    assert_eq!(left, 0);

    ctx.events.finalize_cancel(&id, &BytesN::random(&ctx.env));

    let event = ctx.events.get_event(&id);
    assert_eq!(event.status, EventStatus::Cancelled);
    assert_eq!(event.remaining_escrow, 0);

    for (i, p) in partners.iter().enumerate() {
        assert_eq!(token.balance(p) - balances[i], MIN_CONTRIB);
    }
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
}

#[test]
fn running_non_owner_total_tracks_repeated_contributions_but_not_owner_topups() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    assert_eq!(stored_non_owner_total(&ctx, id), Some(0));

    let owner_top_up = 200_0000000_i128;
    ctx.events
        .add_funds(&id, &ctx.owner, &owner_top_up, &BytesN::random(&ctx.env));
    assert_eq!(
        stored_non_owner_total(&ctx, id),
        Some(0),
        "owner funds are not contributor refund claims"
    );

    let partner = Address::generate(&ctx.env);
    contribute(&ctx, id, &partner, 300_0000000_i128);
    contribute(&ctx, id, &partner, 125_0000000_i128);
    assert_eq!(
        stored_non_owner_total(&ctx, id),
        Some(425_0000000_i128),
        "every credited non-owner top-up is included exactly once"
    );
}

#[test]
fn start_cancel_footprint_is_constant_with_many_contributors() {
    let env = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    let ctx = setup_with_env(env);
    let id = create_hackathon(&ctx);

    for _ in 0..220 {
        let partner = Address::generate(&ctx.env);
        contribute(&ctx, id, &partner, MIN_CONTRIB);
    }

    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));
    let resources = ctx.env.cost_estimate().resources();
    assert!(
        resources.memory_read_entries < 40,
        "start_cancel must read the aggregate, not every contributor: {resources:?}"
    );
    assert!(
        resources.persistent_entry_rent_bumps < 40,
        "TTL work must stay constant as contributor count grows: {resources:?}"
    );
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelling);
}

#[test]
fn non_manager_cranks_and_finalizes_with_exact_payout_deltas() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let manager = Address::generate(&ctx.env);
    ctx.events.set_manager(&id, &manager);
    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    let p1_amount = 200_0000000_i128;
    let p2_amount = 300_0000000_i128;
    contribute(&ctx, id, &p1, p1_amount);
    contribute(&ctx, id, &p2, p2_amount);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    ctx.events.start_cancel(&id, &BytesN::random(&ctx.env));
    let start_auths = ctx.env.auths();
    assert_eq!(start_auths.len(), 1);
    assert_eq!(start_auths[0].0, manager);

    let remaining = ctx
        .events
        .process_cancel_batch(&id, &25_u32, &BytesN::random(&ctx.env));
    assert_eq!(remaining, 0);
    assert!(
        ctx.env.auths().is_empty(),
        "refund cranking must not request authorization"
    );

    ctx.events.finalize_cancel(&id, &BytesN::random(&ctx.env));
    assert!(
        ctx.env.auths().is_empty(),
        "finalization must not request authorization"
    );

    assert_eq!(token.balance(&p1) - p1_before, p1_amount);
    assert_eq!(token.balance(&p2) - p2_before, p2_amount);
    assert_eq!(token.balance(&ctx.owner) - owner_before, TOTAL_BUDGET);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Cancelled);
    assert!(!has_cancellation_state(&ctx, id));
    assert_eq!(stored_non_owner_total(&ctx, id), Some(0));
}

#[test]
fn missing_running_total_initializes_for_an_empty_event() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    remove_running_total(&ctx, id);

    let partner = Address::generate(&ctx.env);
    ctx.token_admin.mint(&partner, &200_0000000_i128);
    ctx.events
        .add_funds(&id, &partner, &100_0000000_i128, &BytesN::random(&ctx.env));
    assert_eq!(stored_non_owner_total(&ctx, id), Some(100_0000000_i128));
}

#[test]
fn missing_running_total_with_contributors_fails_closed() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    let existing_partner = Address::generate(&ctx.env);
    contribute(&ctx, id, &existing_partner, 100_0000000_i128);
    remove_running_total(&ctx, id);

    let partner = Address::generate(&ctx.env);
    ctx.token_admin.mint(&partner, &200_0000000_i128);
    let add_funds_err = ctx
        .events
        .try_add_funds(&id, &partner, &100_0000000_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("missing total rejected")
        .unwrap();
    assert_eq!(add_funds_err, Error::CancellationTotalMissing);

    let cancel_err = ctx
        .events
        .try_start_cancel(&id, &BytesN::random(&ctx.env))
        .err()
        .expect("missing total rejected")
        .unwrap();
    assert_eq!(cancel_err, Error::CancellationTotalMissing);
    assert_eq!(ctx.events.get_event(&id).status, EventStatus::Active);
}

// ============================================================
// ProRataPartners branch (remaining < non_owner_total)
// ============================================================

#[test]
fn cancel_prorata_splits_remaining_across_partners_no_owner_residual() {
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
        title: String::from_str(&ctx.env, "ProRata Cancel"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist,
        fee_bps_override: None,
        manager: None,
    };
    let id = ctx.events.create_event(&params, &BytesN::random(&ctx.env));

    let p1 = Address::generate(&ctx.env);
    let p2 = Address::generate(&ctx.env);
    contribute(&ctx, id, &p1, 500_0000000_i128);
    contribute(&ctx, id, &p2, 500_0000000_i128);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);

    let w = Address::generate(&ctx.env);
    let winners = soroban_sdk::vec![
        &ctx.env,
        WinnerSpec {
            recipient: w.clone(),
            position: 1,
            reputation_bump: 0
        },
    ];
    ctx.events
        .select_winners(&id, &winners, &BytesN::random(&ctx.env));

    // Pull-model: claim prize to drain escrow (60% of 2000 = 1200).
    ctx.events.claim_prize(&id, &w, &1_u32, &0_u32, &BytesN::random(&ctx.env));

    let p1_before = token.balance(&p1);
    let p2_before = token.balance(&p2);
    let owner_before = token.balance(&ctx.owner);
    let fee_before = token.balance(&ctx.fee_account);

    drive_cancel(&ctx.env, &ctx.events, id);

    assert_eq!(token.balance(&p1) - p1_before, 400_0000000_i128);
    assert_eq!(token.balance(&p2) - p2_before, 400_0000000_i128);
    assert_eq!(token.balance(&ctx.owner) - owner_before, 0);
    assert_eq!(token.balance(&ctx.fee_account) - fee_before, 0);
}

// ============================================================
// Error variants
// ============================================================

#[test]
fn start_cancel_on_nonexistent_event_reverts() {
    let ctx = setup();
    assert!(ctx
        .events
        .try_start_cancel(&999_u64, &BytesN::random(&ctx.env))
        .is_err());
}

#[test]
fn start_cancel_on_already_cancelled_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    drive_cancel(&ctx.env, &ctx.events, id);
    assert!(ctx
        .events
        .try_start_cancel(&id, &BytesN::random(&ctx.env))
        .is_err());
}

#[test]
fn process_cancel_batch_without_start_reverts() {
    let ctx = setup();
    let id = create_hackathon(&ctx);
    assert!(ctx
        .events
        .try_process_cancel_batch(&id, &5_u32, &BytesN::random(&ctx.env))
        .is_err());
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
    assert!(ctx
        .events
        .try_finalize_cancel(&id, &BytesN::random(&ctx.env))
        .is_err());
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
    assert!(ctx
        .events
        .try_add_funds(&id, &p2, &MIN_CONTRIB, &BytesN::random(&ctx.env))
        .is_err());
}
