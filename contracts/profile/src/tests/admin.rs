#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, String,
};

use super::common::setup;
use crate::errors::Error;

const EVENTS_CONTRACT_TIMELOCK_LEDGERS: u32 = 17_280;
const PENDING_EVENTS_CONTRACT_TTL_LEDGERS: u32 = 120_960;

const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

#[test]
fn initializes_with_expected_config() {
    let ctx = setup();
    assert_eq!(ctx.client.get_admin(), ctx.admin);
    assert_eq!(ctx.client.is_paused(), false);
    assert_eq!(ctx.client.get_events_contract(), None);
    assert_eq!(ctx.client.get_pending_events_contract(), None);
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.1.0"));
    assert_eq!(ctx.client.get_pending_upgrade(), None);
    assert_eq!(ctx.client.get_migrated_to_version(), None);
}

#[test]
fn pause_and_unpause_round_trip() {
    let ctx = setup();
    ctx.client.pause();
    assert_eq!(ctx.client.is_paused(), true);
    ctx.client.unpause();
    assert_eq!(ctx.client.is_paused(), false);
}

// ============================================================
// EVENTS-CONTRACT ROTATION (H5)
// ============================================================

#[test]
fn first_set_events_contract_succeeds() {
    let ctx = setup();
    let events = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events);
    assert_eq!(ctx.client.get_events_contract(), Some(events));
}

#[test]
fn second_set_events_contract_reverts_already_configured() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    let events_b = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    let err = ctx
        .client
        .try_set_events_contract(&events_b)
        .err()
        .expect("expected second set to fail")
        .unwrap();
    assert_eq!(err, Error::EventsContractAlreadyConfigured);
}

#[test]
fn propose_then_accept_after_timelock_swaps_events_contract() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    let events_b = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    let start = ctx.env.ledger().sequence();
    ctx.client.propose_events_contract(&events_b);

    let pending = ctx
        .client
        .get_pending_events_contract()
        .expect("proposal recorded");
    assert_eq!(pending.target, events_b);
    assert_eq!(pending.proposed_at_ledger, start);

    ctx.env.ledger().with_mut(|li| {
        li.sequence_number = start + EVENTS_CONTRACT_TIMELOCK_LEDGERS + 1;
    });
    ctx.client.accept_events_contract();

    assert_eq!(ctx.client.get_events_contract(), Some(events_b));
    assert_eq!(ctx.client.get_pending_events_contract(), None);
}

#[test]
fn accept_before_timelock_reverts() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    let events_b = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    ctx.client.propose_events_contract(&events_b);
    let err = ctx
        .client
        .try_accept_events_contract()
        .err()
        .expect("expected timelock to block")
        .unwrap();
    assert_eq!(err, Error::PendingEventsContractTimelock);
    assert_eq!(ctx.client.get_events_contract(), Some(events_a));
}

#[test]
fn accept_after_expiry_reverts_and_admin_must_cancel_to_prune() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    let events_b = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    let start = ctx.env.ledger().sequence();
    ctx.client.propose_events_contract(&events_b);

    ctx.env.ledger().with_mut(|li| {
        li.sequence_number = start + PENDING_EVENTS_CONTRACT_TTL_LEDGERS + 1;
    });
    let err = ctx
        .client
        .try_accept_events_contract()
        .err()
        .expect("expected expiry to block")
        .unwrap();
    assert_eq!(err, Error::PendingEventsContractExpired);

    assert!(ctx.client.get_pending_events_contract().is_some());
    assert_eq!(ctx.client.get_events_contract(), Some(events_a));

    ctx.client.cancel_pending_events_contract();
    assert_eq!(ctx.client.get_pending_events_contract(), None);
}

#[test]
fn cancel_pending_clears_proposal() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    let events_b = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    ctx.client.propose_events_contract(&events_b);
    assert!(ctx.client.get_pending_events_contract().is_some());

    ctx.client.cancel_pending_events_contract();
    assert_eq!(ctx.client.get_pending_events_contract(), None);
    assert_eq!(ctx.client.get_events_contract(), Some(events_a));
}

#[test]
fn cancel_with_no_pending_reverts() {
    let ctx = setup();
    let err = ctx
        .client
        .try_cancel_pending_events_contract()
        .err()
        .expect("expected mismatch")
        .unwrap();
    assert_eq!(err, Error::PendingEventsContractMismatch);
}

// ============================================================
// H6: TIMELOCKED UPGRADE + VERSION
// ============================================================

#[test]
fn propose_upgrade_records_pending() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    let before = ctx.env.ledger().sequence();

    ctx.client.propose_upgrade(&new_hash, &new_version);

    let pending = ctx.client.get_pending_upgrade().expect("proposal");
    assert_eq!(pending.wasm_hash, new_hash);
    assert_eq!(pending.new_version, new_version);
    assert_eq!(
        pending.available_at_ledger,
        before + UPGRADE_TIMELOCK_LEDGERS
    );
    assert_eq!(
        pending.expires_at_ledger,
        before + PENDING_UPGRADE_TTL_LEDGERS
    );
}

#[test]
fn apply_upgrade_before_timelock_reverts_profile() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.client.propose_upgrade(&new_hash, &new_version);

    let err = ctx
        .client
        .try_apply_upgrade()
        .err()
        .expect("timelock blocks")
        .unwrap();
    assert_eq!(err, Error::UpgradeTimelockNotElapsed);
}

#[test]
fn apply_upgrade_after_expiry_reverts_profile() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    let start = ctx.env.ledger().sequence();
    ctx.client.propose_upgrade(&new_hash, &new_version);

    ctx.env.ledger().with_mut(|li| {
        li.sequence_number = start + PENDING_UPGRADE_TTL_LEDGERS + 1;
    });

    let err = ctx
        .client
        .try_apply_upgrade()
        .err()
        .expect("expiry blocks")
        .unwrap();
    assert_eq!(err, Error::UpgradeProposalExpired);
}

#[test]
fn migrate_marks_version_and_blocks_replay_profile() {
    let ctx = setup();

    ctx.client.migrate();
    assert_eq!(
        ctx.client.get_migrated_to_version(),
        Some(String::from_str(&ctx.env, "1.1.0"))
    );

    let err = ctx
        .client
        .try_migrate()
        .err()
        .expect("second migrate rejected")
        .unwrap();
    assert_eq!(err, Error::MigrationAlreadyApplied);
}

// ============================================================
// AUTH REGRESSION GUARDS (#73)
//
// Mirror of the events-contract guards in
// contracts/events/src/tests/admin.rs — see that file for the rationale.
// ============================================================

#[test]
fn pause_reverts_without_admin_auth() {
    let ctx = setup();
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_pause();
    assert!(err.is_err(), "pause must require admin auth");
}

#[test]
fn unpause_reverts_without_admin_auth() {
    let ctx = setup();
    ctx.client.pause();
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_unpause();
    assert!(err.is_err(), "unpause must require admin auth");
}

#[test]
fn set_admin_reverts_without_admin_auth() {
    let ctx = setup();
    let new_admin = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_admin(&new_admin);
    assert!(err.is_err(), "set_admin must require admin auth");
}

#[test]
fn set_events_contract_reverts_without_admin_auth() {
    let ctx = setup();
    let events = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_events_contract(&events);
    assert!(err.is_err(), "set_events_contract must require admin auth");
}

#[test]
fn propose_events_contract_reverts_without_admin_auth() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);

    let events_b = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_propose_events_contract(&events_b);
    assert!(
        err.is_err(),
        "propose_events_contract must require admin auth"
    );
}

#[test]
fn accept_events_contract_reverts_without_admin_auth() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);
    let events_b = Address::generate(&ctx.env);
    let start = ctx.env.ledger().sequence();
    ctx.client.propose_events_contract(&events_b);
    ctx.env.ledger().with_mut(|li| {
        li.sequence_number = start + EVENTS_CONTRACT_TIMELOCK_LEDGERS + 1;
    });

    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_accept_events_contract();
    assert!(
        err.is_err(),
        "accept_events_contract must require admin auth"
    );
}

#[test]
fn cancel_pending_events_contract_reverts_without_admin_auth() {
    let ctx = setup();
    let events_a = Address::generate(&ctx.env);
    ctx.client.set_events_contract(&events_a);
    let events_b = Address::generate(&ctx.env);
    ctx.client.propose_events_contract(&events_b);

    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_cancel_pending_events_contract();
    assert!(
        err.is_err(),
        "cancel_pending_events_contract must require admin auth"
    );
}

#[test]
fn propose_upgrade_reverts_without_admin_auth() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_propose_upgrade(&new_hash, &new_version);
    assert!(err.is_err(), "propose_upgrade must require admin auth");
}

#[test]
fn apply_upgrade_reverts_without_admin_auth() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.client.propose_upgrade(&new_hash, &new_version);

    ctx.env.ledger().with_mut(|li| {
        li.sequence_number += UPGRADE_TIMELOCK_LEDGERS;
    });

    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_apply_upgrade();
    assert!(err.is_err(), "apply_upgrade must require admin auth");
}

#[test]
fn cancel_pending_upgrade_reverts_without_admin_auth() {
    let ctx = setup();
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.client.propose_upgrade(&new_hash, &new_version);

    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_cancel_pending_upgrade();
    assert!(
        err.is_err(),
        "cancel_pending_upgrade must require admin auth"
    );
}

#[test]
fn migrate_reverts_without_admin_auth() {
    let ctx = setup();
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_migrate();
    assert!(err.is_err(), "migrate must require admin auth");
}

// ============================================================
// ACCEPT_ADMIN — target-auth guard
//
// accept_admin does not call require_admin(); it authorizes against the
// pending target address instead (pending.target.require_auth()). These
// two tests prove that guard independently of the require_admin() guards
// above: the empty-mock test shows *some* auth is demanded, and the
// auths() check shows it is demanded from the pending target
// specifically, not just any address mock_all_auths() happens to cover.
// ============================================================

#[test]
fn accept_admin_reverts_without_targets_auth() {
    let ctx = setup();
    let new_admin = Address::generate(&ctx.env);
    ctx.client.set_admin(&new_admin);

    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_accept_admin();
    assert!(
        err.is_err(),
        "accept_admin must require the pending target's auth"
    );
}

#[test]
fn accept_admin_demands_pending_targets_auth_specifically() {
    let ctx = setup();
    let new_admin = Address::generate(&ctx.env);
    ctx.client.set_admin(&new_admin);

    ctx.client.accept_admin();

    let auths = ctx.env.auths();
    let target_required = auths.iter().any(|(addr, _)| *addr == new_admin);
    assert!(
        target_required,
        "accept_admin must demand the pending target's own auth"
    );
}
