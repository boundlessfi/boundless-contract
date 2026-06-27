// boundless-profile: credits tests.
//
// Covers contracts/profile/src/credits.rs: bootstrap, spend, earn, refund,
// admin_grant. Happy path + every Error variant reachable from this module
// + edge cases + auth-rejection + idempotency.
//
// Issue: https://github.com/boundlessfi/boundless-contract/issues/26

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, MockAuth, MockAuthInvoke},
    Address, BytesN, IntoVal, String, Symbol,
};

use super::common::setup;
use crate::errors::Error;

fn op_id(env: &soroban_sdk::Env) -> BytesN<32> {
    BytesN::random(env)
}

// ============================================================
// BOOTSTRAP
// ============================================================

#[test]
fn bootstrap_creates_profile_with_default_credits() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);

    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile created");
    assert_eq!(profile.credits, 10);
    assert_eq!(profile.reputation, 0);
    assert_eq!(profile.bootstrapped_at, ctx.env.ledger().timestamp());
}

#[test]
fn bootstrap_is_idempotent_as_a_noop_on_existing_profile() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);

    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    // Spend so we can tell a second bootstrap call didn't reset credits.
    ctx.client
        .spend_credits(&user, &4, &Symbol::new(&ctx.env, "spend"), &op_id(&ctx.env));

    // A different op_id, same user: bootstrap sees the profile already
    // exists and no-ops on the credits field.
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let profile = ctx.client.get_profile(&user).expect("profile still there");
    assert_eq!(profile.credits, 6);
}

#[test]
fn bootstrap_replayed_op_id_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    let id = op_id(&ctx.env);

    ctx.client.bootstrap(&user, &id);
    let err = ctx
        .client
        .try_bootstrap(&user, &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn bootstrap_without_events_contract_configured_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bootstrap(&user, &op_id(&ctx.env))
        .err()
        .expect("missing events contract must fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
fn bootstrap_while_paused_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    ctx.client.pause();
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_bootstrap(&user, &op_id(&ctx.env))
        .err()
        .expect("paused must fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
#[should_panic]
fn bootstrap_called_by_non_events_contract_panics() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    let id = op_id(&ctx.env);

    // Authorize a random address instead of the registered events
    // contract. require_events_contract() calls events.require_auth(),
    // which the host rejects because the wrong address is authorized.
    let impostor = Address::generate(&ctx.env);
    ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "bootstrap",
                args: (user.clone(), id.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .bootstrap(&user, &id);
}

// ============================================================
// SPEND
// ============================================================

#[test]
fn spend_deducts_from_existing_profile() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    ctx.client
        .spend_credits(&user, &3, &Symbol::new(&ctx.env, "apply"), &op_id(&ctx.env));

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 7);
}

#[test]
fn spend_zero_amount_is_a_noop_but_marks_op_seen() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    let id = op_id(&ctx.env);

    ctx.client
        .spend_credits(&user, &0, &Symbol::new(&ctx.env, "noop"), &id);

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 10);

    // Same op_id replayed: still rejected even though the first call
    // touched no balance, because mark_seen runs on the zero-amount path too.
    let err = ctx
        .client
        .try_spend_credits(&user, &0, &Symbol::new(&ctx.env, "noop"), &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);
}

#[test]
fn spend_exact_balance_to_zero_succeeds() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    ctx.client.spend_credits(
        &user,
        &10,
        &Symbol::new(&ctx.env, "apply"),
        &op_id(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 0);
}

#[test]
fn spend_more_than_balance_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let err = ctx
        .client
        .try_spend_credits(
            &user,
            &11,
            &Symbol::new(&ctx.env, "apply"),
            &op_id(&ctx.env),
        )
        .err()
        .expect("overspend must fail")
        .unwrap();
    assert_eq!(err, Error::InsufficientCredits);

    // Balance unchanged after the revert.
    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 10);
}

#[test]
fn spend_on_unbootstrapped_user_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &Symbol::new(&ctx.env, "apply"), &op_id(&ctx.env))
        .err()
        .expect("no profile must fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn spend_replayed_op_id_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    let id = op_id(&ctx.env);

    ctx.client
        .spend_credits(&user, &2, &Symbol::new(&ctx.env, "apply"), &id);
    let err = ctx
        .client
        .try_spend_credits(&user, &2, &Symbol::new(&ctx.env, "apply"), &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    // Only the first call's deduction applied.
    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 8);
}

#[test]
fn spend_while_paused_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    ctx.client.pause();

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &Symbol::new(&ctx.env, "apply"), &op_id(&ctx.env))
        .err()
        .expect("paused must fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn spend_without_events_contract_configured_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_spend_credits(&user, &1, &Symbol::new(&ctx.env, "apply"), &op_id(&ctx.env))
        .err()
        .expect("missing events contract must fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
#[should_panic]
fn spend_called_by_non_events_contract_panics() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let amount: u32 = 1;
    let reason = Symbol::new(&ctx.env, "apply");
    let id = op_id(&ctx.env);
    let impostor = Address::generate(&ctx.env);
    ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "spend_credits",
                args: (user.clone(), amount, reason.clone(), id.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .spend_credits(&user, &amount, &reason, &id);
}

// ============================================================
// EARN
// ============================================================

#[test]
fn earn_adds_to_existing_profile() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    ctx.client
        .earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &op_id(&ctx.env));

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 15);
}

#[test]
fn earn_saturates_instead_of_overflowing() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    ctx.client.earn_credits(
        &user,
        &u32::MAX,
        &Symbol::new(&ctx.env, "win"),
        &op_id(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn earn_on_unbootstrapped_user_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &op_id(&ctx.env))
        .err()
        .expect("no profile must fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn earn_replayed_op_id_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    let id = op_id(&ctx.env);

    ctx.client
        .earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &id);
    let err = ctx
        .client
        .try_earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 15);
}

#[test]
fn earn_while_paused_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    ctx.client.pause();

    let err = ctx
        .client
        .try_earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &op_id(&ctx.env))
        .err()
        .expect("paused must fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn earn_without_events_contract_configured_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_earn_credits(&user, &5, &Symbol::new(&ctx.env, "win"), &op_id(&ctx.env))
        .err()
        .expect("missing events contract must fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
#[should_panic]
fn earn_called_by_non_events_contract_panics() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let amount: u32 = 5;
    let reason = Symbol::new(&ctx.env, "win");
    let id = op_id(&ctx.env);
    let impostor = Address::generate(&ctx.env);
    ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "earn_credits",
                args: (user.clone(), amount, reason.clone(), id.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .earn_credits(&user, &amount, &reason, &id);
}

// ============================================================
// REFUND
// ============================================================

#[test]
fn refund_adds_to_existing_profile() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    ctx.client
        .spend_credits(&user, &6, &Symbol::new(&ctx.env, "apply"), &op_id(&ctx.env));

    ctx.client.refund_credits(
        &user,
        &6,
        &Symbol::new(&ctx.env, "cancelled"),
        &op_id(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 10);
}

#[test]
fn refund_saturates_instead_of_overflowing() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    ctx.client.refund_credits(
        &user,
        &u32::MAX,
        &Symbol::new(&ctx.env, "cancelled"),
        &op_id(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn refund_on_unbootstrapped_user_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_refund_credits(
            &user,
            &5,
            &Symbol::new(&ctx.env, "cancelled"),
            &op_id(&ctx.env),
        )
        .err()
        .expect("no profile must fail")
        .unwrap();
    assert_eq!(err, Error::ProfileNotFound);
}

#[test]
fn refund_replayed_op_id_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    let id = op_id(&ctx.env);

    ctx.client
        .refund_credits(&user, &5, &Symbol::new(&ctx.env, "cancelled"), &id);
    let err = ctx
        .client
        .try_refund_credits(&user, &5, &Symbol::new(&ctx.env, "cancelled"), &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 15);
}

#[test]
fn refund_while_paused_reverts() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    ctx.client.pause();

    let err = ctx
        .client
        .try_refund_credits(
            &user,
            &5,
            &Symbol::new(&ctx.env, "cancelled"),
            &op_id(&ctx.env),
        )
        .err()
        .expect("paused must fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
fn refund_without_events_contract_configured_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_refund_credits(
            &user,
            &5,
            &Symbol::new(&ctx.env, "cancelled"),
            &op_id(&ctx.env),
        )
        .err()
        .expect("missing events contract must fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractNotConfigured);
}

#[test]
#[should_panic]
fn refund_called_by_non_events_contract_panics() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));

    let amount: u32 = 5;
    let reason = Symbol::new(&ctx.env, "cancelled");
    let id = op_id(&ctx.env);
    let impostor = Address::generate(&ctx.env);
    ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "refund_credits",
                args: (user.clone(), amount, reason.clone(), id.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .refund_credits(&user, &amount, &reason, &id);
}

// ============================================================
// ADMIN_GRANT
// ============================================================

#[test]
fn admin_grant_adds_to_existing_profile() {
    let ctx = setup(10);
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    let user = Address::generate(&ctx.env);
    ctx.client.bootstrap(&user, &op_id(&ctx.env));
    // user now has credits = 10 from bootstrap

    ctx.client.admin_grant_credits(
        &user,
        &7,
        &String::from_str(&ctx.env, "support credit"),
        &op_id(&ctx.env),
    );
    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 17);

    ctx.client.admin_grant_credits(
        &user,
        &3,
        &String::from_str(&ctx.env, "extra support credit"),
        &op_id(&ctx.env),
    );
    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 20);
}

#[test]
fn admin_grant_bootstraps_profile_for_unknown_user() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    assert!(ctx.client.get_profile(&user).is_none());

    ctx.client.admin_grant_credits(
        &user,
        &5,
        &String::from_str(&ctx.env, "manual grant"),
        &op_id(&ctx.env),
    );

    // Default bootstrap credits (10) + granted amount (5), since admin_grant
    // bootstraps a fresh Profile (seeded with the default) before adding.
    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 15);
}

#[test]
fn admin_grant_saturates_instead_of_overflowing() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    ctx.client.admin_grant_credits(
        &user,
        &u32::MAX,
        &String::from_str(&ctx.env, "manual grant"),
        &op_id(&ctx.env),
    );

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, u32::MAX);
}

#[test]
fn admin_grant_empty_reason_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_grant_credits(&user, &5, &String::from_str(&ctx.env, ""), &op_id(&ctx.env))
        .err()
        .expect("empty reason must fail")
        .unwrap();
    assert_eq!(err, Error::ReasonRequired);

    // Nothing was bootstrapped on the revert path.
    assert!(ctx.client.get_profile(&user).is_none());
}

#[test]
fn admin_grant_replayed_op_id_reverts() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    let id = op_id(&ctx.env);

    ctx.client
        .admin_grant_credits(&user, &5, &String::from_str(&ctx.env, "manual grant"), &id);
    let err = ctx
        .client
        .try_admin_grant_credits(&user, &5, &String::from_str(&ctx.env, "manual grant"), &id)
        .err()
        .expect("replay must fail")
        .unwrap();
    assert_eq!(err, Error::OpAlreadySeen);

    let profile = ctx.client.get_profile(&user).unwrap();
    assert_eq!(profile.credits, 15);
}

#[test]
fn admin_grant_while_paused_reverts() {
    let ctx = setup(10);
    ctx.client.pause();
    let user = Address::generate(&ctx.env);

    let err = ctx
        .client
        .try_admin_grant_credits(
            &user,
            &5,
            &String::from_str(&ctx.env, "manual grant"),
            &op_id(&ctx.env),
        )
        .err()
        .expect("paused must fail")
        .unwrap();
    assert_eq!(err, Error::Paused);
}

#[test]
#[should_panic]
fn admin_grant_called_by_non_admin_panics() {
    let ctx = setup(10);
    let user = Address::generate(&ctx.env);
    let amount: u32 = 5;
    let reason = String::from_str(&ctx.env, "manual grant");
    let id = op_id(&ctx.env);

    // Authorize a random address instead of the configured admin.
    // require_admin() calls admin.require_auth(), which the host rejects.
    let impostor = Address::generate(&ctx.env);
    ctx.client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "admin_grant_credits",
                args: (user.clone(), amount, reason.clone(), id.clone()).into_val(&ctx.env),
                sub_invokes: &[],
            },
        }])
        .admin_grant_credits(&user, &amount, &reason, &id);
}
