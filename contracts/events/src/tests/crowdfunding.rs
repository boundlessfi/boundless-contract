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

const FUNDING_GOAL: i128 = 1_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
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

    let builder = Address::generate(&env);

    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        builder,
        events_admin,
        fee_account,
        token_addr,
        token_admin,
    }
}

fn single_dist_100_at_1(env: &Env) -> Map<u32, i128> {
    let mut m = Map::new(env);
    m.set(1, 10000000000_i128);
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
        prize_floors: single_dist_100_at_1(&ctx.env),
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

    assert_eq!(token.balance(&ctx.builder), builder_before);

    let event = ctx.events.get_event(&id);
    assert_eq!(event.pillar, Pillar::Crowdfunding);
    assert_eq!(event.total_budget, FUNDING_GOAL);
    assert_eq!(
        event.remaining_escrow, 0,
        "crowdfunding starts with empty escrow"
    );

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
        prize_floors: single_dist_100_at_1(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_create_event(&params, &op);
    assert!(res.is_err(), "single release must be rejected");
}

#[test]
fn create_rejects_floors_above_the_funding_goal() {
    let ctx = setup();
    let mut dist = Map::new(&ctx.env);
    dist.set(1, FUNDING_GOAL);
    dist.set(2, 1_i128);
    let params = CreateEventParams {
        pillar: Pillar::Crowdfunding,
        owner: ctx.builder.clone(),
        token: ctx.token_addr.clone(),
        total_budget: FUNDING_GOAL,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad CF"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        prize_floors: dist,
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
fn claim_milestone_pays_only_the_campaign_owner() {
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 400_0000000_i128);

    let stranger = Address::generate(&ctx.env);
    let err = ctx
        .events
        .try_claim_milestone(&id, &stranger, &0_u32, &0, &BytesN::random(&ctx.env))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, crate::errors::Error::NoSubmissions);

    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &BytesN::random(&ctx.env));
    let paid = ctx.events.get_winner_at(&id, &1).unwrap();
    assert_eq!(paid.recipient, ctx.builder);
    assert_eq!(paid.position, 1);
    assert_eq!(paid.milestone, Some(0));
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
    let ctx = setup();
    let id = create_campaign(&ctx, 1);

    let backer = Address::generate(&ctx.env);
    let pledge = 100_0000000_i128; // exactly 100 USDC, no slack for a fee
    fund(&ctx, &backer, pledge);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    let fee_before = token.balance(&ctx.fee_account);

    let op = BytesN::random(&ctx.env);
    ctx.events.add_funds(&id, &backer, &pledge, &op);

    assert_eq!(token.balance(&backer), 0, "backer pays exactly the pledge");
    assert_eq!(
        token.balance(&ctx.fee_account),
        fee_before,
        "no fee charged at deposit"
    );
    assert_eq!(ctx.events.get_event(&id).remaining_escrow, pledge);

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
        amount: 1_000_0000000_i128,
        reputation_bump: 0,
    };
    let mut winners = SorobanVec::new(&ctx.env);
    winners.push_back(spec);

    let op = BytesN::random(&ctx.env);
    let res = ctx.events.try_select_winners(&id, &winners, &op);
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

#[test]
fn a_release_records_the_fee_it_withheld() {
    use soroban_sdk::{testutils::Events as _, Event as _};

    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let backer = Address::generate(&ctx.env);
    back(&ctx, id, &backer, 900_0000000_i128);

    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0_u32, &BytesN::random(&ctx.env));
    let wanted = crate::events::MilestoneFeeCharged {
        event_id: id,
        recipient: ctx.builder.clone(),
        milestone: 0,
        fee: 7_5000000_i128,
    }
    .to_xdr(&ctx.env, &ctx.events.address);
    let emitted = ctx.env.events().all();
    assert!(emitted
        .filter_by_contract(&ctx.events.address)
        .events()
        .contains(&wanted));
}

fn only_these_sign_the_release(ctx: &Ctx, id: u64, signers: &[&Address], op: &BytesN<32>) {
    only_these_sign_the_release_of(ctx, id, 0, signers, op);
}

fn only_these_sign_the_release_of(
    ctx: &Ctx,
    id: u64,
    milestone: u32,
    signers: &[&Address],
    op: &BytesN<32>,
) {
    extern crate std;
    use soroban_sdk::{
        testutils::{MockAuth, MockAuthInvoke},
        IntoVal,
    };
    let args: SorobanVec<soroban_sdk::Val> =
        (id, ctx.builder.clone(), milestone, 0_u32, op.clone()).into_val(&ctx.env);
    let invoke = MockAuthInvoke {
        contract: &ctx.events.address,
        fn_name: "claim_milestone",
        args,
        sub_invokes: &[],
    };
    let mocks: std::vec::Vec<MockAuth> = signers
        .iter()
        .map(|address| MockAuth {
            address,
            invoke: &invoke,
        })
        .collect();
    ctx.env.mock_auths(&mocks);
}

fn appoint_validator(ctx: &Ctx) -> Address {
    let validator = Address::generate(&ctx.env);
    ctx.events.propose_release_validator(&validator);
    ctx.events.accept_release_validator();
    validator
}

#[test]
fn a_release_validator_co_signs_instead_of_the_admin() {
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let p = Address::generate(&ctx.env);
    back(&ctx, id, &p, 200_0000000_i128);

    let validator = appoint_validator(&ctx);
    assert_eq!(ctx.events.get_release_validator(), Some(validator.clone()));

    let op = BytesN::random(&ctx.env);
    only_these_sign_the_release(&ctx, id, &[&ctx.builder, &ctx.events_admin], &op);
    assert!(ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &0_u32, &0, &op)
        .is_err());

    only_these_sign_the_release(&ctx, id, &[&ctx.builder, &validator], &op);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);
    ctx.env.mock_all_auths_allowing_non_root_auth();
    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.builder), 97_5000000_i128);
}

#[test]
fn a_proposed_validator_co_signs_nothing_until_it_accepts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let p = Address::generate(&ctx.env);
    back(&ctx, id, &p, 200_0000000_i128);

    let proposed = Address::generate(&ctx.env);
    ctx.events.propose_release_validator(&proposed);
    assert_eq!(ctx.events.get_release_validator(), None);
    assert_eq!(
        ctx.events.get_pending_release_validator().unwrap().target,
        proposed
    );

    let op = BytesN::random(&ctx.env);
    only_these_sign_the_release(&ctx, id, &[&ctx.builder, &proposed], &op);
    assert!(ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &0_u32, &0, &op)
        .is_err());
    only_these_sign_the_release(&ctx, id, &[&ctx.builder, &ctx.events_admin], &op);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);
}

#[test]
fn only_the_proposed_key_can_accept() {
    use soroban_sdk::{
        testutils::{MockAuth, MockAuthInvoke},
        IntoVal,
    };
    let ctx = setup();
    let proposed = Address::generate(&ctx.env);
    ctx.events.propose_release_validator(&proposed);

    for signer in [&ctx.events_admin, &ctx.builder] {
        ctx.env.mock_auths(&[MockAuth {
            address: signer,
            invoke: &MockAuthInvoke {
                contract: &ctx.events.address,
                fn_name: "accept_release_validator",
                args: ().into_val(&ctx.env),
                sub_invokes: &[],
            },
        }]);
        assert!(ctx.events.try_accept_release_validator().is_err());
    }
    ctx.env.mock_auths(&[MockAuth {
        address: &proposed,
        invoke: &MockAuthInvoke {
            contract: &ctx.events.address,
            fn_name: "accept_release_validator",
            args: ().into_val(&ctx.env),
            sub_invokes: &[],
        },
    }]);
    ctx.events.accept_release_validator();
    ctx.env.mock_all_auths_allowing_non_root_auth();
    assert_eq!(ctx.events.get_release_validator(), Some(proposed));
    assert!(ctx.events.get_pending_release_validator().is_none());
}

#[test]
fn only_the_admin_proposes_cancels_or_clears() {
    use soroban_sdk::{
        testutils::{MockAuth, MockAuthInvoke},
        IntoVal,
    };
    let ctx = setup();
    let target = Address::generate(&ctx.env);
    let as_builder = |fn_name: &'static str, args: SorobanVec<soroban_sdk::Val>| {
        ctx.env.mock_auths(&[MockAuth {
            address: &ctx.builder,
            invoke: &MockAuthInvoke {
                contract: &ctx.events.address,
                fn_name,
                args,
                sub_invokes: &[],
            },
        }]);
    };
    as_builder(
        "propose_release_validator",
        (target.clone(),).into_val(&ctx.env),
    );
    assert!(ctx.events.try_propose_release_validator(&target).is_err());

    ctx.env.mock_all_auths_allowing_non_root_auth();
    ctx.events.propose_release_validator(&target);
    as_builder("cancel_pending_release_validator", ().into_val(&ctx.env));
    assert!(ctx.events.try_cancel_pending_release_validator().is_err());
    as_builder("clear_release_validator", ().into_val(&ctx.env));
    assert!(ctx.events.try_clear_release_validator().is_err());
    ctx.env.mock_all_auths_allowing_non_root_auth();
    assert!(ctx.events.get_pending_release_validator().is_some());
}

#[test]
fn an_unaccepted_proposal_expires_or_can_be_withdrawn() {
    use soroban_sdk::testutils::Ledger as _;
    let ctx = setup();
    let target = Address::generate(&ctx.env);

    ctx.events.propose_release_validator(&target);
    ctx.events.cancel_pending_release_validator();
    assert_eq!(
        ctx.events
            .try_accept_release_validator()
            .err()
            .unwrap()
            .unwrap(),
        crate::errors::Error::PendingRotationMismatch
    );
    assert_eq!(
        ctx.events
            .try_cancel_pending_release_validator()
            .err()
            .unwrap()
            .unwrap(),
        crate::errors::Error::PendingRotationMismatch
    );

    ctx.events.propose_release_validator(&target);
    let expires = ctx
        .events
        .get_pending_release_validator()
        .unwrap()
        .expires_at_ledger;
    ctx.env
        .ledger()
        .with_mut(|l| l.sequence_number = expires + 1);
    assert_eq!(
        ctx.events
            .try_accept_release_validator()
            .err()
            .unwrap()
            .unwrap(),
        crate::errors::Error::PendingRotationExpired
    );
    assert_eq!(ctx.events.get_release_validator(), None);
}

#[test]
fn replacing_a_validator_keeps_the_old_one_until_the_new_one_accepts() {
    let ctx = setup();
    let id = create_campaign(&ctx, 3);
    let p = Address::generate(&ctx.env);
    back(&ctx, id, &p, 300_0000000_i128);
    let old = appoint_validator(&ctx);

    let new = Address::generate(&ctx.env);
    ctx.events.propose_release_validator(&new);
    let op = BytesN::random(&ctx.env);
    only_these_sign_the_release(&ctx, id, &[&ctx.builder, &old], &op);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &op);

    ctx.env.mock_all_auths_allowing_non_root_auth();
    ctx.events.accept_release_validator();
    let op = BytesN::random(&ctx.env);
    only_these_sign_the_release_of(&ctx, id, 1, &[&ctx.builder, &old], &op);
    assert!(ctx
        .events
        .try_claim_milestone(&id, &ctx.builder, &1_u32, &0, &op)
        .is_err());
    only_these_sign_the_release_of(&ctx, id, 1, &[&ctx.builder, &new], &op);
    ctx.events
        .claim_milestone(&id, &ctx.builder, &1_u32, &0, &op);
}

#[test]
fn clearing_the_validator_returns_co_signing_to_the_admin_at_once() {
    let ctx = setup();
    let id = create_campaign(&ctx, 2);
    let p = Address::generate(&ctx.env);
    back(&ctx, id, &p, 200_0000000_i128);
    appoint_validator(&ctx);
    ctx.events
        .propose_release_validator(&Address::generate(&ctx.env));

    ctx.events.clear_release_validator();
    assert_eq!(ctx.events.get_release_validator(), None);
    assert!(ctx.events.get_pending_release_validator().is_none());

    ctx.events
        .claim_milestone(&id, &ctx.builder, &0_u32, &0, &BytesN::random(&ctx.env));
    let auths = ctx.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == ctx.events_admin));
}
