// boundless-profile: earnings registration tests.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN,
};

use super::common::setup;
use crate::errors::Error;

fn configure_events_contract(ctx: &super::common::TestCtx<'_>) -> Address {
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    events
}

#[test]
fn register_earnings_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&user, &token, &100_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("events contract missing")
        .unwrap();

    assert_eq!(err, Error::EventsContractNotConfigured);
    assert_eq!(ctx.client.get_earnings(&user, &token), 0);
}

#[test]
fn register_earnings_records_amount_for_token() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    ctx.client
        .register_earnings(&user, &token, &250_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&user, &token), 250);
}

#[test]
fn register_earnings_accumulates_per_user_and_token() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    let other_token = Address::generate(&ctx.env);

    ctx.client
        .register_earnings(&user, &token, &100_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&user, &token, &75_i128, &BytesN::random(&ctx.env));
    ctx.client
        .register_earnings(&user, &other_token, &30_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&user, &token), 175);
    assert_eq!(ctx.client.get_earnings(&user, &other_token), 30);
}

#[test]
fn register_earnings_rejects_zero_amount() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&user, &token, &0_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("zero amount rejected")
        .unwrap();

    assert_eq!(err, Error::InvalidAmount);
    assert_eq!(ctx.client.get_earnings(&user, &token), 0);
}

#[test]
fn register_earnings_rejects_negative_amount() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&user, &token, &-1_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("negative amount rejected")
        .unwrap();

    assert_eq!(err, Error::InvalidAmount);
    assert_eq!(ctx.client.get_earnings(&user, &token), 0);
}

#[test]
fn register_earnings_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client
        .register_earnings(&user, &token, &100_i128, &op_id);
    let err = ctx
        .client
        .try_register_earnings(&user, &token, &100_i128, &op_id)
        .err()
        .expect("replayed op rejected")
        .unwrap();

    assert_eq!(err, Error::OpAlreadySeen);
    assert_eq!(ctx.client.get_earnings(&user, &token), 100);
}

#[test]
fn register_earnings_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    ctx.client.pause();
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_register_earnings(&user, &token, &100_i128, &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract rejected")
        .unwrap();

    assert_eq!(err, Error::Paused);
    assert_eq!(ctx.client.get_earnings(&user, &token), 0);
}

#[test]
fn register_earnings_requires_events_contract_auth() {
    let ctx = setup(10);
    let events = configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);

    ctx.client
        .register_earnings(&user, &token, &100_i128, &BytesN::random(&ctx.env));

    let events_required = ctx.env.auths().iter().any(|(addr, _)| *addr == events);
    assert!(
        events_required,
        "register_earnings must demand events contract authorization"
    );
}
