// boundless-profile: earnings tests (#30).
//
// Covers register_earnings:
//   - Happy path + each Error variant + edge cases + auth-rejection + idempotency.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN,
};

use super::common::setup;
use crate::errors::Error;

const BOOTSTRAP: u32 = 10;

// ============================================================
// register_earnings
// ============================================================

#[test]
fn register_earnings_accumulates() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.register_earnings(&user, &token, &1_000_0000000_i128, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&user, &token), 1_000_0000000_i128);

    ctx.client.register_earnings(&user, &token, &500_0000000_i128, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&user, &token), 1_500_0000000_i128);
}

#[test]
fn register_earnings_zero_amount_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let err = ctx.client
        .try_register_earnings(&user, &token, &0_i128, &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

#[test]
fn register_earnings_negative_amount_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let err = ctx.client
        .try_register_earnings(&user, &token, &-1_i128, &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::InvalidAmount);
}

#[test]
fn register_earnings_different_tokens_tracked_independently() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token_a = Address::generate(&ctx.env);
    let token_b = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.register_earnings(&user, &token_a, &100_0000000_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&user, &token_b, &200_0000000_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&user, &token_a), 100_0000000_i128);
    assert_eq!(ctx.client.get_earnings(&user, &token_b), 200_0000000_i128);
}

#[test]
fn register_earnings_different_users_tracked_independently() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user_a = Address::generate(&ctx.env);
    let user_b = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user_a, &BytesN::random(&ctx.env));
    ctx.client.bootstrap(&user_b, &BytesN::random(&ctx.env));

    ctx.client.register_earnings(&user_a, &token, &300_0000000_i128, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&user_b, &token, &700_0000000_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&user_a, &token), 300_0000000_i128);
    assert_eq!(ctx.client.get_earnings(&user_b, &token), 700_0000000_i128);
}

#[test]
fn register_earnings_saturates_at_i128_max() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.register_earnings(&user, &token, &i128::MAX, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&user, &token, &1_i128, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_earnings(&user, &token), i128::MAX);
}

#[test]
fn register_earnings_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let op = BytesN::random(&ctx.env);
    ctx.client.register_earnings(&user, &token, &100_0000000_i128, &op);
    assert!(ctx.client.try_register_earnings(&user, &token, &100_0000000_i128, &op).is_err());
}

#[test]
fn get_earnings_returns_zero_for_unknown_user_token() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    assert_eq!(ctx.client.get_earnings(&user, &token), 0);
}

#[test]
fn register_earnings_succeeds_when_events_contract_is_authorized() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));
    ctx.client.register_earnings(&user, &token, &50_0000000_i128, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_earnings(&user, &token), 50_0000000_i128);
}

#[test]
fn register_earnings_without_events_contract_auth_reverts() {
    use soroban_sdk::testutils::MockAuth;
    use soroban_sdk::testutils::MockAuthInvoke;
    use soroban_sdk::IntoVal;

    let ctx = setup(BOOTSTRAP);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);

    let user = Address::generate(&ctx.env);
    let token = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let op = BytesN::random(&ctx.env);
    let impostor = Address::generate(&ctx.env);
    let result = ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "register_earnings",
                args: (user.clone(), token.clone(), 50_0000000_i128, op.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .try_register_earnings(&user, &token, &50_0000000_i128, &op);
    assert!(result.is_err());
}
