// boundless-events: token whitelist tests (#27).
//
// Covers register_supported_token / deregister_supported_token /
// is_supported_token + enforcement inside create_event.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, Env, Map, String,
};

use crate::types::{CreateEventParams, Pillar, ReleaseKind};
use crate::{EventsContract, EventsContractClient};

const FEE_BPS: u32 = 250;

struct Ctx<'a> {
    env: Env,
    admin: Address,
    client: EventsContractClient<'a>,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let profile = Address::generate(&env);
    let id = env.register(
        EventsContract,
        (admin.clone(), fee_account.clone(), FEE_BPS, profile.clone()),
    );
    let client = EventsContractClient::new(&env, &id);
    Ctx { env, admin, client }
}

fn new_token(env: &Env) -> Address {
    let issuer = Address::generate(env);
    env.register_stellar_asset_contract_v2(issuer).address()
}

fn single_dist(env: &Env) -> Map<u32, u32> {
    let mut m = Map::new(env);
    m.set(1, 100);
    m
}

// ============================================================
// register_supported_token
// ============================================================

#[test]
fn register_makes_token_supported() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    assert!(!ctx.client.is_supported_token(&tok));
    ctx.client.register_supported_token(&tok);
    assert!(ctx.client.is_supported_token(&tok));
}

#[test]
fn register_same_token_twice_is_idempotent() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.register_supported_token(&tok);
    ctx.client.register_supported_token(&tok);
    assert!(ctx.client.is_supported_token(&tok));
}

#[test]
fn register_requires_admin_auth() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.register_supported_token(&tok);
    let auths = ctx.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == ctx.admin));
}

// ============================================================
// deregister_supported_token
// ============================================================

#[test]
fn deregister_removes_support() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.register_supported_token(&tok);
    ctx.client.deregister_supported_token(&tok);
    assert!(!ctx.client.is_supported_token(&tok));
}

#[test]
fn deregister_unregistered_token_is_idempotent() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.deregister_supported_token(&tok);
    assert!(!ctx.client.is_supported_token(&tok));
}

#[test]
fn deregister_requires_admin_auth() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.register_supported_token(&tok);
    ctx.client.deregister_supported_token(&tok);
    let auths = ctx.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == ctx.admin));
}

// ============================================================
// Enforcement: create_event rejects unsupported token
// ============================================================

#[test]
fn create_event_with_unsupported_token_reverts() {
    let ctx = setup();
    let owner = Address::generate(&ctx.env);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner,
        token: new_token(&ctx.env),
        total_budget: 1_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://example.com"),
        title: String::from_str(&ctx.env, "Bad Token Hack"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let res = ctx.client.try_create_event(&params, &BytesN::random(&ctx.env));
    assert!(res.is_err());
}

#[test]
fn create_event_with_deregistered_token_reverts() {
    let ctx = setup();
    let tok = new_token(&ctx.env);
    ctx.client.register_supported_token(&tok);
    ctx.client.deregister_supported_token(&tok);
    let owner = Address::generate(&ctx.env);
    let params = CreateEventParams {
        pillar: Pillar::Hackathon,
        owner,
        token: tok,
        total_budget: 1_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(&ctx.env, "https://example.com"),
        title: String::from_str(&ctx.env, "Deregistered Token Hack"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        winner_distribution: single_dist(&ctx.env),
        application_credit_cost: 0,
        fee_bps_override: None,
        manager: None,
    };
    let res = ctx.client.try_create_event(&params, &BytesN::random(&ctx.env));
    assert!(res.is_err());
}

// ============================================================
// Multiple tokens tracked independently
// ============================================================

#[test]
fn multiple_tokens_registered_independently() {
    let ctx = setup();
    let a = new_token(&ctx.env);
    let b = new_token(&ctx.env);

    ctx.client.register_supported_token(&a);
    assert!(ctx.client.is_supported_token(&a));
    assert!(!ctx.client.is_supported_token(&b));

    ctx.client.register_supported_token(&b);
    ctx.client.deregister_supported_token(&a);
    assert!(!ctx.client.is_supported_token(&a));
    assert!(ctx.client.is_supported_token(&b));
}
