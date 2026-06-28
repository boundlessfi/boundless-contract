// boundless-events: token whitelist enumeration tests.
//
// The whitelist is readable from state (count + at) so the full set can be
// recovered authoritatively, independent of ephemeral event retention.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Vec};

use super::common::{setup, TestCtx};

fn snapshot(ctx: &TestCtx<'_>) -> Vec<Address> {
    let count = ctx.client.supported_token_count();
    let mut out = Vec::new(&ctx.env);
    let mut i = 0;
    while i < count {
        out.push_back(ctx.client.supported_token_at(&i).expect("indexed token"));
        i += 1;
    }
    out
}

fn has(set: &Vec<Address>, token: &Address) -> bool {
    set.iter().any(|t| &t == token)
}

#[test]
fn register_indexes_the_token() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);

    assert_eq!(ctx.client.supported_token_count(), 1);
    assert_eq!(ctx.client.supported_token_at(&0), Some(token.clone()));
    assert_eq!(ctx.client.supported_token_at(&1), None);
    assert!(ctx.client.is_supported_token(&token));
}

#[test]
fn register_is_idempotent_in_the_index() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);
    ctx.client.register_supported_token(&token);

    assert_eq!(ctx.client.supported_token_count(), 1);
}

#[test]
fn deregister_removes_from_the_index() {
    let ctx = setup(250);
    let token = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&token);
    ctx.client.deregister_supported_token(&token);

    assert_eq!(ctx.client.supported_token_count(), 0);
    assert_eq!(ctx.client.supported_token_at(&0), None);
    assert!(!ctx.client.is_supported_token(&token));
}

#[test]
fn deregister_unknown_token_is_a_noop() {
    let ctx = setup(250);
    let a = Address::generate(&ctx.env);
    let b = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&a);
    // `b` was never registered; removing it must not corrupt the index.
    ctx.client.deregister_supported_token(&b);

    assert_eq!(ctx.client.supported_token_count(), 1);
    assert_eq!(ctx.client.supported_token_at(&0), Some(a));
}

#[test]
fn enumerates_multiple_and_swap_removes_the_middle() {
    let ctx = setup(250);
    let a = Address::generate(&ctx.env);
    let b = Address::generate(&ctx.env);
    let c = Address::generate(&ctx.env);

    ctx.client.register_supported_token(&a);
    ctx.client.register_supported_token(&b);
    ctx.client.register_supported_token(&c);
    assert_eq!(ctx.client.supported_token_count(), 3);

    // Remove the middle entry; swap-with-last keeps the set intact (order may
    // change, membership must not).
    ctx.client.deregister_supported_token(&b);
    assert_eq!(ctx.client.supported_token_count(), 2);

    let set = snapshot(&ctx);
    assert!(has(&set, &a));
    assert!(has(&set, &c));
    assert!(!has(&set, &b));
    assert!(ctx.client.is_supported_token(&a));
    assert!(!ctx.client.is_supported_token(&b));
    assert!(ctx.client.is_supported_token(&c));
}
