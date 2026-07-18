#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, String, Symbol,
};

use super::common::{setup, TestCtx};
use crate::errors::Error;

// ============================================================
// Helpers
// ============================================================

fn op_id(ctx: &TestCtx) -> BytesN<32> {
    BytesN::random(&ctx.env)
}

fn reason(ctx: &TestCtx) -> Symbol {
    Symbol::new(&ctx.env, "win")
}

fn reputation_of(ctx: &TestCtx, user: &Address) -> u64 {
    ctx.client
        .get_profile(user)
        .expect("profile exists")
        .reputation
}

fn setup_with_user<'a>() -> (TestCtx<'a>, Address) {
    let ctx = setup();
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);

    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));
    (ctx, user)
}

// ============================================================
// bump
// ============================================================

#[test]
fn bump_happy_path_increments_reputation() {
    let (ctx, user) = setup_with_user();
    assert_eq!(reputation_of(&ctx, &user), 0);

    ctx.client
        .bump_reputation(&user, &5, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 5);
}

#[test]
fn bump_accumulates_across_calls() {
    let (ctx, user) = setup_with_user();

    ctx.client
        .bump_reputation(&user, &5, &reason(&ctx), &op_id(&ctx));
    ctx.client
        .bump_reputation(&user, &7, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 12);
}

#[test]
fn bump_accepts_u32_max_delta_without_overflow() {
    let (ctx, user) = setup_with_user();

    ctx.client
        .bump_reputation(&user, &u32::MAX, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), u32::MAX as u64);
}

#[test]
fn bump_zero_delta_is_noop_but_marks_seen() {
    let (ctx, user) = setup_with_user();
    let op = op_id(&ctx);

    ctx.client.bump_reputation(&user, &0, &reason(&ctx), &op);
    assert_eq!(reputation_of(&ctx, &user), 0);

    let err = ctx
        .client
        .try_bump_reputation(&user, &0, &reason(&ctx), &op)
        .err()
        .expect("replay rejected")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn bump_reverts_when_events_contract_not_configured() {
    let ctx = setup();
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected guard to reject")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn bump_reverts_when_paused() {
    let (ctx, user) = setup_with_user();
    ctx.client.pause();

    let err = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected pause to block")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn bump_reverts_when_profile_not_found() {
    let ctx = setup();
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);

    let ghost = Address::generate(&ctx.env);
    let err = ctx
        .client
        .try_bump_reputation(&ghost, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected missing profile")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn bump_is_idempotent_on_replay() {
    let (ctx, user) = setup_with_user();
    let op = op_id(&ctx);

    ctx.client.bump_reputation(&user, &5, &reason(&ctx), &op);
    assert_eq!(reputation_of(&ctx, &user), 5);

    let err = ctx
        .client
        .try_bump_reputation(&user, &5, &reason(&ctx), &op)
        .err()
        .expect("replay rejected")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
    assert_eq!(reputation_of(&ctx, &user), 5);
}

#[test]
fn bump_rejects_caller_without_events_contract_auth() {
    let (ctx, user) = setup_with_user();

    ctx.env.mock_auths(&[]);
    let res = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &op_id(&ctx));
    assert!(res.is_err(), "unauthorized bump must be rejected");
}

// ============================================================
// slash
// ============================================================

#[test]
fn slash_happy_path_decrements_reputation() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &op_id(&ctx));

    ctx.client
        .slash_reputation(&user, &4, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 6);
}

#[test]
fn slash_saturates_at_zero() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &5, &reason(&ctx), &op_id(&ctx));

    ctx.client
        .slash_reputation(&user, &10, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 0);
}

#[test]
fn slash_zero_delta_is_noop() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &3, &reason(&ctx), &op_id(&ctx));

    ctx.client
        .slash_reputation(&user, &0, &reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 3);
}

#[test]
fn slash_reverts_when_events_contract_not_configured() {
    let ctx = setup();
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected guard to reject")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn slash_reverts_when_paused() {
    let (ctx, user) = setup_with_user();
    ctx.client.pause();

    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected pause to block")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn slash_reverts_when_profile_not_found() {
    let ctx = setup();
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);

    let ghost = Address::generate(&ctx.env);
    let err = ctx
        .client
        .try_slash_reputation(&ghost, &1, &reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected missing profile")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn slash_is_idempotent_on_replay() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &op_id(&ctx));
    let op = op_id(&ctx);

    ctx.client.slash_reputation(&user, &4, &reason(&ctx), &op);
    assert_eq!(reputation_of(&ctx, &user), 6);

    let err = ctx
        .client
        .try_slash_reputation(&user, &4, &reason(&ctx), &op)
        .err()
        .expect("replay rejected")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
    assert_eq!(reputation_of(&ctx, &user), 6);
}

#[test]
fn slash_rejects_caller_without_events_contract_auth() {
    let (ctx, user) = setup_with_user();

    ctx.env.mock_auths(&[]);
    let res = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &op_id(&ctx));
    assert!(res.is_err(), "unauthorized slash must be rejected");
}

// ============================================================
// admin_slash
// ============================================================

fn admin_reason(ctx: &TestCtx) -> String {
    String::from_str(&ctx.env, "fraud")
}

#[test]
fn admin_slash_happy_path_decrements_reputation() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &op_id(&ctx));

    ctx.client
        .admin_slash_reputation(&user, &3, &admin_reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 7);
}

#[test]
fn admin_slash_saturates_at_zero() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &2, &reason(&ctx), &op_id(&ctx));

    ctx.client
        .admin_slash_reputation(&user, &9, &admin_reason(&ctx), &op_id(&ctx));

    assert_eq!(reputation_of(&ctx, &user), 0);
}

#[test]
fn admin_slash_reverts_on_empty_reason() {
    let (ctx, user) = setup_with_user();
    let empty = String::from_str(&ctx.env, "");

    let err = ctx
        .client
        .try_admin_slash_reputation(&user, &1, &empty, &op_id(&ctx))
        .err()
        .expect("expected empty reason to reject")
        .unwrap();
    assert_eq!(err, Error::ReasonRequired);
}

#[test]
fn admin_slash_reverts_when_paused() {
    let (ctx, user) = setup_with_user();
    ctx.client.pause();

    let err = ctx
        .client
        .try_admin_slash_reputation(&user, &1, &admin_reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected pause to block")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn admin_slash_reverts_when_profile_not_found() {
    let ctx = setup();
    let ghost = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_slash_reputation(&ghost, &1, &admin_reason(&ctx), &op_id(&ctx))
        .err()
        .expect("expected missing profile")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn admin_slash_is_idempotent_on_replay() {
    let (ctx, user) = setup_with_user();
    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &op_id(&ctx));
    let op = op_id(&ctx);

    ctx.client
        .admin_slash_reputation(&user, &3, &admin_reason(&ctx), &op);
    assert_eq!(reputation_of(&ctx, &user), 7);

    let err = ctx
        .client
        .try_admin_slash_reputation(&user, &3, &admin_reason(&ctx), &op)
        .err()
        .expect("replay rejected")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
    assert_eq!(reputation_of(&ctx, &user), 7);
}

#[test]
fn admin_slash_rejects_non_admin_caller() {
    let (ctx, user) = setup_with_user();

    ctx.env.mock_auths(&[]);
    let res = ctx
        .client
        .try_admin_slash_reputation(&user, &1, &admin_reason(&ctx), &op_id(&ctx));
    assert!(res.is_err(), "non-admin admin_slash must be rejected");
}

#[test]
fn admin_slash_demands_admins_auth_specifically() {
    // Complements admin_slash_rejects_non_admin_caller above (already in
    // the codebase since #60): that test proves *some* auth is required;
    // this one proves the auth demanded under a normal call is
    // specifically the admin's, not just any address mock_all_auths()
    // happens to approve.
    let (ctx, user) = setup_with_user();
    ctx.client
        .admin_slash_reputation(&user, &1, &admin_reason(&ctx), &op_id(&ctx));

    let auths = ctx.env.auths();
    let admin_required = auths.iter().any(|(addr, _)| *addr == ctx.admin);
    assert!(admin_required, "admin_slash must demand the admin's auth");
}
