// boundless-profile: reputation tests.
//
// Covers reputation::bump, reputation::slash, reputation::admin_slash.
// Every function: happy path + each reachable Error variant + edge cases
// (saturating add/sub, zero delta) + auth-rejection + idempotency replay.
//
// Spec: boundless-credits-reputation-prd.md Section 5.3.

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

/// A fresh, unique idempotency key.
fn op_id(ctx: &TestCtx) -> BytesN<32> {
    BytesN::random(&ctx.env)
}

/// bump/slash reason (Symbol).
fn reason(ctx: &TestCtx) -> Symbol {
    Symbol::new(&ctx.env, "win")
}

/// Current reputation for a user that is expected to have a profile.
fn reputation_of(ctx: &TestCtx, user: &Address) -> u64 {
    ctx.client
        .get_profile(user)
        .expect("profile exists")
        .reputation
}

/// setup() + wire an events contract + bootstrap one user so the
/// events-gated reputation ops have a profile to mutate.
///
/// Returns the context plus the bootstrapped user. The events-contract
/// address is mocked-authed by `setup`, so subsequent bump/slash calls
/// satisfy `require_events_contract`.
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
    // delta is u32, reputation is u64. A single max-delta bump must widen
    // cleanly into u64 and never overflow/panic.
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

    // Replaying the same op_id is rejected even though the op was a no-op.
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
    // No set_events_contract: the events-contract auth guard is the first
    // check and rejects before anything else.
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

    // A user that was never bootstrapped has no profile.
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
    // Reputation unchanged: the replay did not double-apply.
    assert_eq!(reputation_of(&ctx, &user), 5);
}

#[test]
fn bump_rejects_caller_without_events_contract_auth() {
    // Genuine auth rejection: the events contract is configured, but no
    // authorization is provided for the bump call, so events.require_auth()
    // fails and the host aborts the invocation.
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
    // Slashing more than the current reputation floors at zero rather than
    // underflowing (saturating_sub).
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

/// admin_slash reason is a String (audited free text), not a Symbol.
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
    // require_admin passes (mocked), then the pause guard fires.
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
    // No events contract needed: admin_slash is admin-gated, not events-gated.
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
    // Genuine auth rejection: no authorization provided, so admin.require_auth()
    // fails and the host aborts the invocation.
    let (ctx, user) = setup_with_user();

    ctx.env.mock_auths(&[]);
    let res = ctx
        .client
        .try_admin_slash_reputation(&user, &1, &admin_reason(&ctx), &op_id(&ctx));
    assert!(res.is_err(), "non-admin admin_slash must be rejected");
}
