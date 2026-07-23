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

#[test]
fn register_earnings_succeeds() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client.register_earnings(&u, &t, &100_i128, &op_id);

    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

#[test]
fn register_earnings_accumulates() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    ctx.client
        .register_earnings(&u, &t, &50_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&u, &t, &30_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&u, &t, &20_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

#[test]
fn register_earnings_multiple_tokens() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t1 = token(&ctx.env);
    let t2 = token(&ctx.env);

    ctx.client
        .register_earnings(&u, &t1, &100_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&u, &t2, &200_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u, &t1), 100);
    assert_eq!(ctx.client.get_earnings(&u, &t2), 200);
}

#[test]
fn register_earnings_multiple_users() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u1 = user(&ctx.env);
    let u2 = user(&ctx.env);
    let t = token(&ctx.env);

    ctx.client
        .register_earnings(&u1, &t, &100_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&u2, &t, &200_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&u1, &t), 100);
    assert_eq!(ctx.client.get_earnings(&u2, &t), 200);
}

#[test]
fn register_earnings_rejects_zero() {
    let ctx = setup();
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
    let ctx = setup();
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

#[test]
fn register_earnings_reverts_no_events_contract() {
    let ctx = setup();
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

#[test]
fn register_earnings_reverts_when_paused() {
    let ctx = setup();
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

#[test]
fn register_earnings_rejects_duplicate_op_id() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client.register_earnings(&u, &t, &100_i128, &op_id);
    assert_eq!(ctx.client.get_earnings(&u, &t), 100);

    let err = ctx
        .client
        .try_register_earnings(&u, &t, &200_i128, &op_id)
        .err()
        .expect("call should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    assert_eq!(ctx.client.get_earnings(&u, &t), 100);
}

#[test]
fn register_earnings_saturating_add() {
    let ctx = setup();
    ctx.client.set_events_contract(&events_addr(&ctx.env));

    let u = user(&ctx.env);
    let t = token(&ctx.env);

    ctx.client
        .register_earnings(&u, &t, &(i128::MAX - 1), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&u, &t), i128::MAX - 1);

    ctx.client
        .register_earnings(&u, &t, &100_i128, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&u, &t), i128::MAX);
}

#[test]
fn register_earnings_auth_rejection() {
    let env = soroban_sdk::Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(crate::ProfileContract, (admin.clone(),));
    let client = crate::ProfileContractClient::new(&env, &contract_id);

    env.as_contract(&contract_id, || {
        crate::storage::set_events_contract(&env, &events_addr(&env));
    });

    let u = user(&env);
    let t = token(&env);

    let result = client.try_register_earnings(&u, &t, &100_i128, &BytesN::random(&env));
    assert!(
        result.is_err(),
        "expected auth failure when not called by events contract"
    );
}
