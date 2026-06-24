// boundless-profile: reputation tests (#29).
//
// Covers bump_reputation / slash_reputation / admin_slash_reputation:
//   - Happy path + each Error variant + edge cases + auth-rejection + idempotency.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, String, Symbol,
};

use super::common::setup;
use crate::errors::Error;

const BOOTSTRAP: u32 = 10;

fn reason(env: &soroban_sdk::Env) -> Symbol {
    Symbol::new(env, "test")
}

fn setup_with_profile<'a>() -> (super::common::TestCtx<'a>, Address) {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));
    (ctx, user)
}

// ============================================================
// bump_reputation
// ============================================================

#[test]
fn bump_increments_reputation() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &100_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 100);
}

#[test]
fn bump_multiple_times_accumulates() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &50_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    ctx.client.bump_reputation(&user, &25_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 75);
}

#[test]
fn bump_large_values_accumulate_without_overflow() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &u32::MAX, &reason(&ctx.env), &BytesN::random(&ctx.env));
    ctx.client.bump_reputation(&user, &u32::MAX, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, (u32::MAX as u64) * 2);
}

#[test]
fn bump_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    let err = ctx.client
        .try_bump_reputation(&user, &10_u32, &reason(&ctx.env), &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn bump_op_replay_reverts() {
    let (ctx, user) = setup_with_profile();
    let op = BytesN::random(&ctx.env);
    ctx.client.bump_reputation(&user, &10_u32, &reason(&ctx.env), &op);
    assert!(ctx.client.try_bump_reputation(&user, &10_u32, &reason(&ctx.env), &op).is_err());
}

// ============================================================
// slash_reputation
// ============================================================

#[test]
fn slash_decrements_reputation() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &100_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    ctx.client.slash_reputation(&user, &30_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 70);
}

#[test]
fn slash_saturates_at_zero() {
    let (ctx, user) = setup_with_profile();
    ctx.client.slash_reputation(&user, &u32::MAX, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 0);
}

#[test]
fn slash_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    assert!(ctx.client.try_slash_reputation(&user, &10_u32, &reason(&ctx.env), &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn slash_op_replay_reverts() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &50_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    let op = BytesN::random(&ctx.env);
    ctx.client.slash_reputation(&user, &10_u32, &reason(&ctx.env), &op);
    assert!(ctx.client.try_slash_reputation(&user, &10_u32, &reason(&ctx.env), &op).is_err());
}

// ============================================================
// admin_slash_reputation
// ============================================================

#[test]
fn admin_slash_decrements_reputation() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &100_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));

    let r = String::from_str(&ctx.env, "rule violation");
    ctx.client.admin_slash_reputation(&user, &40_u32, &r, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 60);
}

#[test]
fn admin_slash_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);
    let r = String::from_str(&ctx.env, "reason");
    let err = ctx.client
        .try_admin_slash_reputation(&user, &10_u32, &r, &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn admin_slash_empty_reason_reverts() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &50_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    let empty = String::from_str(&ctx.env, "");
    let err = ctx.client
        .try_admin_slash_reputation(&user, &10_u32, &empty, &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::ReasonRequired);
}

#[test]
fn admin_slash_requires_admin_auth() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &100_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    let r = String::from_str(&ctx.env, "violation");
    ctx.client.admin_slash_reputation(&user, &10_u32, &r, &BytesN::random(&ctx.env));
    let auths = ctx.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == ctx.admin));
}

#[test]
fn admin_slash_op_replay_reverts() {
    let (ctx, user) = setup_with_profile();
    ctx.client.bump_reputation(&user, &100_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    let r = String::from_str(&ctx.env, "reason");
    let op = BytesN::random(&ctx.env);
    ctx.client.admin_slash_reputation(&user, &10_u32, &r, &op);
    assert!(ctx.client.try_admin_slash_reputation(&user, &10_u32, &r, &op).is_err());
}

#[test]
fn admin_slash_saturates_at_zero() {
    let (ctx, user) = setup_with_profile();
    let r = String::from_str(&ctx.env, "reason");
    ctx.client.admin_slash_reputation(&user, &u32::MAX, &r, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().reputation, 0);
}
