//! Security regression for issue #68:
//! - XOR-malleable child op_id derivation
//! - global OpSeen squat via bootstrap_self (payout DoS)

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::idempotency::{self, tag};
use crate::types::{CreateEventParams, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const FEE_BPS: u32 = 250;
const TOTAL_BUDGET: i128 = 10_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    events_id: Address,
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
        events_id,
        profile,
        owner,
        applicant,
        token_addr,
    }
}

fn dist_100(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

fn create_bounty(ctx: &Ctx) -> u64 {
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/events/op-id-sec"),
        title: String::from_str(&ctx.env, "OpId Security"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: dist_100(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    ctx.events.create_event(&params, &BytesN::random(&ctx.env))
}

/// Two parents that collide under the old XOR scheme must yield distinct children.
#[test]
fn sha256_child_ids_differ_for_xor_colliding_parents() {
    let ctx = setup();
    let env = &ctx.env;

    // Distinct parents that only differ in byte0 (the old XOR mutation surface).
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    a[0] = 0x10;
    a[1] = 0x20;
    a[2] = 0xAA;
    b[0] = 0x10 ^ tag::BOOTSTRAP;
    b[1] = 0x20;
    b[2] = 0xAA;
    let parent_a = BytesN::from_array(env, &a);
    let parent_b = BytesN::from_array(env, &b);
    assert_ne!(parent_a, parent_b);

    // derive_child reads profile contract storage — must run as the events contract.
    let (child_a, child_b, child_rep, child_i0, child_i1) = env.as_contract(&ctx.events_id, || {
        let child_a = idempotency::derive_child(env, &parent_a, tag::BOOTSTRAP);
        let child_b = idempotency::derive_child(env, &parent_b, tag::BOOTSTRAP);
        let child_rep = idempotency::derive_child(env, &parent_a, tag::BUMP_REP);
        let child_i0 = idempotency::derive_child_indexed(env, &parent_a, tag::BOOTSTRAP, 0);
        let child_i1 = idempotency::derive_child_indexed(env, &parent_a, tag::BOOTSTRAP, 1);
        (child_a, child_b, child_rep, child_i0, child_i1)
    });

    assert_ne!(
        child_a, child_b,
        "distinct parents must produce distinct sha256 children"
    );
    assert_ne!(child_a, child_rep);
    assert_eq!(child_a, child_i0);
    assert_ne!(child_i0, child_i1);
    assert_ne!(child_a, parent_a);
}

/// Attacker squats derived child ids via bootstrap_self; legitimate claim_prize still pays.
#[test]
fn bootstrap_self_cannot_front_run_events_child_op_ids() {
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
            reputation_bump: 50,
        },
    ];
    let op_select = BytesN::random(&ctx.env);
    ctx.events.select_winners(&bounty_id, &winners, &op_select);

    // Parent op_id the winner will use for claim_prize — attacker observes it and
    // pre-marks the derived profile child ids via unprivileged bootstrap_self.
    let claim_op = BytesN::random(&ctx.env);
    let (bootstrap_child, rep_child, earnings_child) = ctx.env.as_contract(&ctx.events_id, || {
        (
            idempotency::derive_child(&ctx.env, &claim_op, tag::BOOTSTRAP),
            idempotency::derive_child(&ctx.env, &claim_op, tag::BUMP_REP),
            idempotency::derive_child(&ctx.env, &claim_op, tag::REGISTER_EARNINGS),
        )
    });

    let attacker = Address::generate(&ctx.env);
    ctx.profile.bootstrap_self(&attacker, &bootstrap_child);
    ctx.profile.bootstrap_self(&attacker, &rep_child);
    ctx.profile.bootstrap_self(&attacker, &earnings_child);

    // Legitimate claim must still succeed: OpSeen is namespaced by events domain.
    ctx.events.claim_prize(&bounty_id, &1_u32, &claim_op);

    let token = token::Client::new(&ctx.env, &ctx.token_addr);
    assert_eq!(token.balance(&ctx.applicant), TOTAL_BUDGET);

    let profile = ctx.profile.get_profile(&ctx.applicant).unwrap();
    assert_eq!(profile.reputation, 50);

    let earnings = ctx.profile.get_earnings(&ctx.applicant, &ctx.token_addr);
    assert_eq!(earnings, TOTAL_BUDGET);

    // Sanity: attacker got a profile from bootstrap_self, but that is independent.
    assert!(ctx.profile.get_profile(&attacker).is_some());
}

/// True replay of the same events-domain child op_id is still rejected.
#[test]
fn events_domain_child_op_id_replay_still_rejected() {
    let ctx = setup();
    let env = &ctx.env;

    // Bootstrap via events path twice with the same child id.
    let parent = BytesN::random(env);
    let child = env.as_contract(&ctx.events_id, || {
        idempotency::derive_child(env, &parent, tag::BOOTSTRAP)
    });
    let user = Address::generate(env);

    ctx.profile.bootstrap(&user, &child);
    let replay = ctx.profile.try_bootstrap(&user, &child);
    assert!(
        replay.is_err(),
        "true replay of the same events-domain op_id must be rejected"
    );
}
