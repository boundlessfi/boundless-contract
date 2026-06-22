// boundless-events: token whitelist tests.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, Env, Map, String,
};

use super::common::setup;
use crate::errors::Error;
use crate::types::{CreateEventParams, Pillar, ReleaseKind};

fn one_winner(env: &Env) -> Map<u32, u32> {
    let mut distribution = Map::new(env);
    distribution.set(1, 100);
    distribution
}

fn bounty_params(env: &Env, owner: &Address, token: &Address) -> CreateEventParams {
    CreateEventParams {
        pillar: Pillar::Bounty,
        owner: owner.clone(),
        token: token.clone(),
        total_budget: 1_000_0000000_i128,
        release_kind: ReleaseKind::Single,
        content_uri: String::from_str(env, "ipfs://bounty"),
        title: String::from_str(env, "Whitelist gated bounty"),
        deadline: Some(env.ledger().timestamp() + 10_000),
        winner_distribution: one_winner(env),
        application_credit_cost: 1,
        fee_bps_override: None,
    }
}

#[test]
fn token_is_unsupported_by_default() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    assert_eq!(ctx.client.is_supported_token(&token), false);
}

#[test]
fn register_marks_token_supported() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);

    assert_eq!(ctx.client.is_supported_token(&token), true);
}

#[test]
fn deregister_marks_token_unsupported() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);
    ctx.client.deregister_supported_token(&token);

    assert_eq!(ctx.client.is_supported_token(&token), false);
}

#[test]
fn deregister_unregistered_token_is_idempotent() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.deregister_supported_token(&token);
    ctx.client.deregister_supported_token(&token);

    assert_eq!(ctx.client.is_supported_token(&token), false);
}

#[test]
fn create_event_rejects_unregistered_token() {
    let ctx = setup(250);
    let owner = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    let params = bounty_params(&ctx.env, &owner, &token);

    let err = ctx
        .client
        .try_create_event(&params, &BytesN::random(&ctx.env))
        .err()
        .expect("unsupported token rejected")
        .unwrap();

    assert_eq!(err, Error::TokenNotSupported);
}

#[test]
fn register_supported_token_requires_admin_auth() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);

    let admin_required = ctx.env.auths().iter().any(|(addr, _)| *addr == ctx.admin);
    assert!(
        admin_required,
        "register_supported_token must demand admin authorization"
    );
}

#[test]
fn deregister_supported_token_requires_admin_auth() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.deregister_supported_token(&token);

    let admin_required = ctx.env.auths().iter().any(|(addr, _)| *addr == ctx.admin);
    assert!(
        admin_required,
        "deregister_supported_token must demand admin authorization"
    );
}
