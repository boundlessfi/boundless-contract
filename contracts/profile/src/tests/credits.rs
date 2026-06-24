// boundless-profile: credits tests (#26).
//
// Covers spend / earn / refund / admin_grant:
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

// ============================================================
// bootstrap
// ============================================================

#[test]
fn bootstrap_creates_profile_with_initial_credits() {
    let ctx = setup(BOOTSTRAP);
    let ec = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&ec);

    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let p = ctx.client.get_profile(&user).unwrap();
    assert_eq!(p.credits, BOOTSTRAP);
    assert_eq!(p.reputation, 0);
}

#[test]
fn bootstrap_is_idempotent() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let p = ctx.client.get_profile(&user).unwrap();
    assert_eq!(p.credits, BOOTSTRAP);
}

#[test]
fn bootstrap_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));

    let user = Address::generate(&ctx.env);
    let op = BytesN::random(&ctx.env);
    ctx.client.bootstrap(&user, &op);
    assert!(ctx.client.try_bootstrap(&user, &op).is_err());
}

// ============================================================
// spend_credits
// ============================================================

#[test]
fn spend_decrements_credits() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.spend_credits(&user, &3_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    let p = ctx.client.get_profile(&user).unwrap();
    assert_eq!(p.credits, BOOTSTRAP - 3);
}

#[test]
fn spend_zero_is_no_op() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.spend_credits(&user, &0_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, BOOTSTRAP);
}

#[test]
fn spend_insufficient_credits_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let err = ctx.client
        .try_spend_credits(&user, &(BOOTSTRAP + 1), &reason(&ctx.env), &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::InsufficientCredits);
}

#[test]
fn spend_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    let err = ctx.client
        .try_spend_credits(&user, &1_u32, &reason(&ctx.env), &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn spend_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let op = BytesN::random(&ctx.env);
    ctx.client.spend_credits(&user, &1_u32, &reason(&ctx.env), &op);
    assert!(ctx.client.try_spend_credits(&user, &1_u32, &reason(&ctx.env), &op).is_err());
}

// ============================================================
// earn_credits
// ============================================================

#[test]
fn earn_increments_credits() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.earn_credits(&user, &5_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, BOOTSTRAP + 5);
}

#[test]
fn earn_saturates_at_max_u32() {
    let ctx = setup(u32::MAX);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.earn_credits(&user, &1_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, u32::MAX);
}

#[test]
fn earn_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    assert!(ctx.client.try_earn_credits(&user, &5_u32, &reason(&ctx.env), &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn earn_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let op = BytesN::random(&ctx.env);
    ctx.client.earn_credits(&user, &5_u32, &reason(&ctx.env), &op);
    assert!(ctx.client.try_earn_credits(&user, &5_u32, &reason(&ctx.env), &op).is_err());
}

// ============================================================
// refund_credits
// ============================================================

#[test]
fn refund_increments_credits() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    ctx.client.spend_credits(&user, &3_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    ctx.client.refund_credits(&user, &2_u32, &reason(&ctx.env), &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, BOOTSTRAP - 3 + 2);
}

#[test]
fn refund_on_missing_profile_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    assert!(ctx.client.try_refund_credits(&user, &1_u32, &reason(&ctx.env), &BytesN::random(&ctx.env)).is_err());
}

#[test]
fn refund_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let op = BytesN::random(&ctx.env);
    ctx.client.refund_credits(&user, &1_u32, &reason(&ctx.env), &op);
    assert!(ctx.client.try_refund_credits(&user, &1_u32, &reason(&ctx.env), &op).is_err());
}

// ============================================================
// admin_grant_credits
// ============================================================

#[test]
fn admin_grant_increments_credits_on_existing_profile() {
    let ctx = setup(BOOTSTRAP);
    ctx.client.set_events_contract(&Address::generate(&ctx.env));
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));

    let reason_str = String::from_str(&ctx.env, "campaign bonus");
    ctx.client.admin_grant_credits(&user, &50_u32, &reason_str, &BytesN::random(&ctx.env));
    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, BOOTSTRAP + 50);
}

#[test]
fn admin_grant_creates_profile_when_missing() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);

    let reason_str = String::from_str(&ctx.env, "first grant");
    ctx.client.admin_grant_credits(&user, &20_u32, &reason_str, &BytesN::random(&ctx.env));
    let p = ctx.client.get_profile(&user).unwrap();
    assert_eq!(p.credits, BOOTSTRAP + 20);
}

#[test]
fn admin_grant_empty_reason_reverts() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);
    let empty = String::from_str(&ctx.env, "");
    let err = ctx.client
        .try_admin_grant_credits(&user, &10_u32, &empty, &BytesN::random(&ctx.env))
        .err().unwrap().unwrap();
    assert_eq!(err, Error::ReasonRequired);
}

#[test]
fn admin_grant_requires_admin_auth() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);
    let reason_str = String::from_str(&ctx.env, "grant");
    ctx.client.admin_grant_credits(&user, &5_u32, &reason_str, &BytesN::random(&ctx.env));
    let auths = ctx.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == ctx.admin));
}

#[test]
fn admin_grant_op_replay_reverts() {
    let ctx = setup(BOOTSTRAP);
    let user = Address::generate(&ctx.env);
    let reason_str = String::from_str(&ctx.env, "grant");
    let op = BytesN::random(&ctx.env);
    ctx.client.admin_grant_credits(&user, &5_u32, &reason_str, &op);
    assert!(ctx.client.try_admin_grant_credits(&user, &5_u32, &reason_str, &op).is_err());
}
