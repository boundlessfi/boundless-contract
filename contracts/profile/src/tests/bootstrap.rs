// boundless-profile: self-service bootstrap tests.
//
// `bootstrap_self` is the user-authorized profile-creation path used at
// platform onboarding. Unlike `bootstrap` (events-contract-gated) it requires
// NO admin key and NO events contract — the profile owner authorizes their own
// creation. These tests pin that security property plus idempotency.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN,
};

use super::common::setup;
use crate::errors::Error;

#[test]
fn bootstrap_self_creates_profile_for_caller() {
    let ctx = setup(30);
    let user = Address::generate(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    assert_eq!(ctx.client.get_profile(&user), None);
    ctx.client.bootstrap_self(&user, &op_id);

    let profile = ctx.client.get_profile(&user).expect("profile created");
    assert_eq!(profile.credits, 30);
    assert_eq!(profile.reputation, 0);
}

#[test]
fn bootstrap_self_demands_the_callers_own_auth_not_admin() {
    // The security property the design rests on: bootstrap_self requires the
    // USER's own authorization — no admin or other privileged key can create a
    // profile on someone's behalf. mock_all_auths lets the call through, but
    // env.auths() records whose auth the contract actually demanded.
    let ctx = setup(30);
    let user = Address::generate(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client.bootstrap_self(&user, &op_id);

    let auths = ctx.env.auths();
    assert!(
        auths.iter().any(|(addr, _)| *addr == user),
        "bootstrap_self must demand the user's own auth"
    );
    assert!(
        !auths.iter().any(|(addr, _)| *addr == ctx.admin),
        "bootstrap_self must NOT demand admin auth"
    );
}

#[test]
fn bootstrap_self_is_idempotent_for_existing_profile() {
    let ctx = setup(30);
    let user = Address::generate(&ctx.env);

    ctx.client.bootstrap_self(&user, &BytesN::random(&ctx.env));
    // Second bootstrap (fresh op_id) when the profile already exists is a
    // no-op — credits are not re-granted.
    ctx.client.bootstrap_self(&user, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_profile(&user).unwrap().credits, 30);
}

#[test]
fn bootstrap_self_rejects_a_replayed_op_id() {
    let ctx = setup(30);
    let user = Address::generate(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    ctx.client.bootstrap_self(&user, &op_id);
    let err = ctx
        .client
        .try_bootstrap_self(&user, &op_id)
        .err()
        .expect("a replayed op_id must be rejected")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}
