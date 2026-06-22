// boundless-profile: credits tests.

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
    Symbol::new(&ctx.env, "credits")
}

fn admin_reason(ctx: &TestCtx<'_>) -> String {
    String::from_str(&ctx.env, "manual-adjustment")
}

#[test]
fn spend_credits_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing events contract should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn spend_credits_requires_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn spend_credits_rejects_insufficient_balance() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    let err = ctx
        .client
        .try_spend_credits(&user, &11, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("insufficient balance should fail")
        .unwrap();
    assert_eq!(err, Error::InsufficientCredits);
}

#[test]
fn spend_credits_decreases_balance() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .spend_credits(&user, &4, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, 6);
}

#[test]
fn spend_credits_zero_amount_is_noop_but_marks_op_seen() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);
    let op = BytesN::random(&ctx.env);

    ctx.client.spend_credits(&user, &0, &reason(&ctx), &op);
    assert_eq!(ctx.client.get_profile(&user), None);

    let err = ctx
        .client
        .try_spend_credits(&user, &0, &reason(&ctx), &op)
        .err()
        .expect("replayed noop op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn spend_credits_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    let op = BytesN::random(&ctx.env);

    ctx.client.spend_credits(&user, &2, &reason(&ctx), &op);
    let err = ctx
        .client
        .try_spend_credits(&user, &1, &reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn spend_credits_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn spend_credits_requires_events_contract_auth() {
    let ctx = setup(10);
    let events = configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail after auth")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == events),
        "spend must demand events contract auth"
    );
}

#[test]
fn earn_credits_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_earn_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing events contract should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn earn_credits_requires_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_earn_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn earn_credits_increases_balance() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .earn_credits(&user, &5, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, 15);
}

#[test]
fn earn_credits_saturates_at_u32_max() {
    let ctx = setup(0);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .earn_credits(&user, &u32::MAX, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client
        .earn_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn earn_credits_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    let op = BytesN::random(&ctx.env);

    ctx.client.earn_credits(&user, &2, &reason(&ctx), &op);
    let err = ctx
        .client
        .try_earn_credits(&user, &1, &reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn earn_credits_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_earn_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn refund_credits_requires_configured_events_contract() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_refund_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing events contract should fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn refund_credits_requires_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_refund_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("missing profile should fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn refund_credits_increases_balance() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .refund_credits(&user, &5, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, 15);
}

#[test]
fn refund_credits_saturates_at_u32_max() {
    let ctx = setup(0);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .refund_credits(&user, &u32::MAX, &reason(&ctx), &BytesN::random(&ctx.env));
    ctx.client
        .refund_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn refund_credits_rejects_replayed_op_id() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    let op = BytesN::random(&ctx.env);

    ctx.client.refund_credits(&user, &2, &reason(&ctx), &op);
    let err = ctx
        .client
        .try_refund_credits(&user, &1, &reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn refund_credits_rejects_when_paused() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);
    ctx.client.pause();

    let err = ctx
        .client
        .try_refund_credits(&user, &1, &reason(&ctx), &BytesN::random(&ctx.env))
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn admin_grant_credits_requires_nonempty_reason() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_grant_credits(
            &user,
            &1,
            &String::from_str(&ctx.env, ""),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("empty admin grant reason should fail")
        .unwrap();
    assert_eq!(err, Error::ReasonRequired);
}

#[test]
fn admin_grant_credits_creates_missing_profile_with_default_balance() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    ctx.client
        .admin_grant_credits(&user, &5, &admin_reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile created");
    assert_eq!(profile.credits, 15);
}

#[test]
fn admin_grant_credits_adds_to_existing_profile() {
    let ctx = setup(10);
    configure_events_contract(&ctx);
    let user = bootstrap_user(&ctx);

    ctx.client
        .admin_grant_credits(&user, &5, &admin_reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, 15);
}

#[test]
fn admin_grant_credits_saturates_at_u32_max() {
    let ctx = setup(0);
    let user = Address::generate(&ctx.env);

    ctx.client.admin_grant_credits(
        &user,
        &u32::MAX,
        &admin_reason(&ctx),
        &BytesN::random(&ctx.env),
    );
    ctx.client
        .admin_grant_credits(&user, &1, &admin_reason(&ctx), &BytesN::random(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile exists");
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn admin_grant_credits_rejects_replayed_op_id() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    let op = BytesN::random(&ctx.env);

    ctx.client
        .admin_grant_credits(&user, &5, &admin_reason(&ctx), &op);
    let err = ctx
        .client
        .try_admin_grant_credits(&user, &1, &admin_reason(&ctx), &op)
        .err()
        .expect("replayed op id should fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn admin_grant_credits_rejects_when_paused() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    ctx.client.pause();

    let err = ctx
        .client
        .try_admin_grant_credits(
            &user,
            &1,
            &admin_reason(&ctx),
            &BytesN::random(&ctx.env),
        )
        .err()
        .expect("paused contract should fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn admin_grant_credits_requires_admin_auth() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    ctx.client
        .admin_grant_credits(&user, &5, &admin_reason(&ctx), &BytesN::random(&ctx.env));

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == ctx.admin),
        "admin grant must demand admin auth"
    );
}
