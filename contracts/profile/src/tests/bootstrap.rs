#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN,
};

use super::common::setup;
use crate::errors::Error;

#[test]
fn bootstrap_self_creates_profile_for_caller() {
    let ctx = setup();
    let user = Address::generate(&ctx.env);
    let op_id = BytesN::random(&ctx.env);

    assert_eq!(ctx.client.get_profile(&user), None);
    ctx.client.bootstrap_self(&user, &op_id);

    let profile = ctx.client.get_profile(&user).expect("profile created");
    assert_eq!(profile.reputation, 0);
}

#[test]
fn bootstrap_self_demands_the_callers_own_auth_not_admin() {
    let ctx = setup();
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
    let ctx = setup();
    let user = Address::generate(&ctx.env);

    ctx.client.bootstrap_self(&user, &BytesN::random(&ctx.env));
    let before = ctx.client.get_profile(&user).expect("profile created");

    ctx.client.bootstrap_self(&user, &BytesN::random(&ctx.env));

    assert_eq!(ctx.client.get_profile(&user), Some(before));
}

#[test]
fn bootstrap_self_rejects_a_replayed_op_id() {
    let ctx = setup();
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
