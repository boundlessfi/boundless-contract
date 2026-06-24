// boundless-profile: earnings registration tests.
//
// Covers register_earnings() — guards, error variants, idempotency,
// auth rejection, saturating arithmetic.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN,
};

use super::common::setup;
use crate::errors::Error;

fn events_addr(env: &crate::Env) -> Address {
    Address::generate(env)
}

fn user(env: &crate::Env) -> Address {
    Address::generate(env)
}

fn token(env: &crate::Env) -> Address {
    Address::generate(env)
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_succeeds() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client.register_earnings(&u, &t, &100_i128, &op_id);

    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

#[test]
fn register_earnings_accumulates() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    ctx.client.register_earnings(&u, &t, &50_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&u, &t, &30_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&u, &t, &20_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

// ---------------------------------------------------------------------------
// Edge cases: multiple tokens / users
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_multiple_tokens() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t1 = token(&ctx.env);
    let t2 = token(&ctx.env);

    ctx.client.register_earnings(&u, &t1, &100_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&u, &t2, &200_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u, &t1), 100);
    assert_eq!(ctx.client.get_earnings(&u, &t2), 200);
}

#[test]
fn register_earnings_multiple_users() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u1 = user(&ctx.env);
    let u2 = user(&ctx.env);
    let t = token(&ctx.env);

    ctx.client.register_earnings(&u1, &t, &100_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&u2, &t, &200_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u1, &t), 100);
    assert_eq!(ctx.client.get_earnings(&u2, &t), 200);
}

// ---------------------------------------------------------------------------
// Error: InvalidAmount (zero / negative)
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_rejects_zero() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&u, &t, &0_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

#[test]
fn register_earnings_rejects_negative() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&u, &t, &(-100_i128), &BytesN::random(&ctx.env))
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

// ---------------------------------------------------------------------------
// Error: EventsContractNotConfigured
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_reverts_no_events_contract() {
    let ctx = setup(10);
    let u = user(&ctx.env);
    let t = token(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&u, &t, &100_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

// ---------------------------------------------------------------------------
// Error: Paused
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_reverts_when_paused() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));
    ctx.client.pause();

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&u, &t, &100_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

// ---------------------------------------------------------------------------
// Idempotency: duplicate op_id
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_rejects_duplicate_op_id() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    // First call succeeds.
    ctx.client.register_earnings(&u, &t, &100_i128, &op_id);
    assert_eq!(ctx.client.get_earnings(&u, &t), 100);

    // Same op_id — idempotency guard.
    let err = ctx
        .client
        .try_register_earnings(&u, &t, &200_i128, &op_id)
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    // Balance unchanged.
    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

// ---------------------------------------------------------------------------
// Edge: saturating arithmetic
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_saturating_add() {
    let ctx = setup(10);
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    // Push to i128::MAX - 1.
    ctx.client.register_earnings(
        &u,
        &t,
        &(i128::MAX - 1),
        &BytesN::random(&ctx.env),
    );
    assert_eq!(ctx.client.get_earnings(&u, &t), i128::MAX - 1);

    // Add 100 — should saturate at i128::MAX, not overflow.
    ctx.client.register_earnings(&u, &t, &100_i128, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&u, &t), i128::MAX);
}

// ---------------------------------------------------------------------------
// Auth rejection: caller is not the events contract.
// ---------------------------------------------------------------------------

#[test]
fn register_earnings_auth_rejection() {
    let env = soroban_sdk::Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(
        crate::ProfileContract,
        (admin.clone(), 10u32),
    );
    let client = crate::ProfileContractClient::new(&env, &contract_id);

    // Set events contract directly in storage to bypass admin auth.
    env.as_contract(&contract_id, || {
        crate::storage::set_events_contract(&env, &events_addr(&env));
    });

    let u = user(&env);
    let t = token(&env);

    // No mock_auths — the events address hasn't authorized this call,
    // so require_auth() inside register_earnings should fail.
    let result = client.try_register_earnings(&u, &t, &100_i128, &BytesN::random(&env));
    assert!(
        result.is_err(),
        "expected auth failure when not called by events contract"
    );
}
