// boundless-profile: reputation tests.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, String, Symbol,
};

use super::common::{setup, TestCtx};
use crate::errors::Error;

fn configure_events_contract(ctx: &TestCtx<'_>) -> Address {
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    events
}

fn bootstrap_user(ctx: &TestCtx<'_>) -> Address {
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &BytesN::random(&ctx.env));
    user
}

fn reason(ctx: &TestCtx<'_>) -> Symbol {
    Symbol::new(&ctx.env, "milestone")
}

#[test]
fn bump_reputation_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing events contract should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn bump_reputation_requires_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn bump_reputation_increases_score() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .bump_reputation(&user, &7, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.reputation, 7);
}

#[test]
fn bump_reputation_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    let op = BytesN::random(&ctx.env);

    ctx.client.bump_reputation(&user, &7, &reason(&ctx), &op);
    let err = ctx
        .client
        .try_bump_reputation(&user, &7, &reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn bump_reputation_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_bump_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn bump_reputation_requires_events_contract_auth() {
    let ctx = setup(10);
    let events = configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bump_reputation(&user, &3, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail after auth")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == events),
        "reputation bump must demand events contract auth"
    );
}

#[test]
fn slash_reputation_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing events contract should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn slash_reputation_requires_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn slash_reputation_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn slash_reputation_reduces_score() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .bump_reputation(&user, &9, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client
        .slash_reputation(&user, &4, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.reputation, 5);
}

#[test]
fn slash_reputation_saturates_at_zero() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .bump_reputation(&user, &5, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client
        .slash_reputation(&user, &99, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.reputation, 0);
}

#[test]
fn slash_reputation_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client
        .bump_reputation(&user, &5, &reason(&ctx), &BytesN::random(&ctx.env));
    let op = BytesN::random(&ctx.env);

    ctx.client.slash_reputation(&user, &1, &reason(&ctx), &op);
    let err = ctx
        .client
        .try_slash_reputation(&user, &1, &reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn admin_slash_reputation_requires_nonempty_reason() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    let err = ctx
        .client
        .try_admin_slash_reputation(
            &user,
            &1,
            &String::from_str(&ctx.env, ""),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("empty admin slash reason should fail")
        .unwrap();
    assert_eq!(err, Error::ReasonRequired);
}

#[test]
fn admin_slash_reputation_requires_existing_profile() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_slash_reputation(
            &user,
            &1,
            &String::from_str(&ctx.env, "manual-review"),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn admin_slash_reputation_reduces_score() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client.admin_slash_reputation(
        &user,
        &6,
        &String::from_str(&ctx.env, "manual-review"),
        &BytesN::random(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.reputation, 4);
}

#[test]
fn admin_slash_reputation_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client
        .bump_reputation(&user, &10, &reason(&ctx), &BytesN::random(&ctx.env));
    let op = BytesN::random(&ctx.env);

    ctx.client.admin_slash_reputation(
        &user,
        &1,
        &String::from_str(&ctx.env, "manual-review"),
        &op,
    );
    let err = ctx
        .client
        .try_admin_slash_reputation(
            &user,
            &1,
            &String::from_str(&ctx.env, "manual-review"),
            &op,
        )
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn admin_slash_reputation_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_admin_slash_reputation(
            &user,
            &1,
            &String::from_str(&ctx.env, "manual-review"),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn admin_slash_reputation_saturates_at_zero() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .bump_reputation(&user, &3, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client.admin_slash_reputation(
        &user,
        &99,
        &String::from_str(&ctx.env, "manual-review"),
        &BytesN::random(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.reputation, 0);
}

#[test]
fn admin_slash_reputation_requires_admin_auth() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_slash_reputation(
            &user,
            &1,
            &String::from_str(&ctx.env, "manual-review"),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("missing profile should fail after auth")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == ctx.admin),
        "admin slash must demand admin auth"
    );
}
