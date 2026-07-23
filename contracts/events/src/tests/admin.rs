#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, String,
};

use super::common::setup;
use crate::errors::Error;

const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

#[test]
fn initializes_with_expected_config() {
    let ctx = setup(250);
    assert_eq!(ctx.client.get_admin(), ctx.admin);
    assert_eq!(ctx.client.get_fee_account(), ctx.fee_account);
    assert_eq!(ctx.client.get_fee_bps(), 250);
    assert_eq!(ctx.client.get_profile_contract(), ctx.profile_contract);
    assert_eq!(ctx.client.is_paused(), false);
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.4.0"));
    assert_eq!(ctx.client.get_pending_upgrade(), None);
    assert_eq!(ctx.client.get_migrated_to_version(), None);
}

#[test]
fn pause_and_unpause_round_trip() {
    let ctx = setup(250);
    ctx.client.pause();
    assert_eq!(ctx.client.is_paused(), true);
    ctx.client.unpause();
    assert_eq!(ctx.client.is_paused(), false);
}

#[test]
fn id_base_encodes_deployment_sequence() {
    let ctx = setup(250);
    let base = ctx.client.id_base();
    assert_eq!(base & 0xFFFF_FFFF, 0);
}

// ============================================================
// H6: TIMELOCKED UPGRADE + VERSION
// ============================================================

#[test]
fn propose_upgrade_records_pending_and_emits() {
    let ctx = setup(250);
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    let before = ctx.env.ledger().sequence();

    ctx.client.propose_upgrade(&new_hash, &new_version);

    let pending = ctx.client.get_pending_upgrade().expect("proposal");
    assert_eq!(pending.wasm_hash, new_hash);
    assert_eq!(pending.new_version, new_version);
    assert_eq!(pending.proposed_at_ledger, before);
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
fn propose_upgrade_rejects_empty_version() {
    let ctx = setup(250);
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let empty = String::from_str(&ctx.env, "");

    let err = ctx
        .client
        .try_propose_upgrade(&new_hash, &empty)
        .err()
        .expect("empty version rejected")
        .unwrap();
    assert_eq!(err, Error::InvalidPillar);
}

#[test]
fn apply_upgrade_before_timelock_reverts() {
    let ctx = setup(250);
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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.4.0"));
}

#[test]
fn apply_upgrade_after_expiry_reverts() {
    let ctx = setup(250);
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
fn cancel_pending_upgrade_clears_proposal() {
    let ctx = setup(250);
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.client.propose_upgrade(&new_hash, &new_version);
    assert!(ctx.client.get_pending_upgrade().is_some());

    ctx.client.cancel_pending_upgrade();
    assert_eq!(ctx.client.get_pending_upgrade(), None);
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.4.0"));
}

#[test]
fn cancel_with_no_pending_reverts() {
    let ctx = setup(250);
    let err = ctx
        .client
        .try_cancel_pending_upgrade()
        .err()
        .expect("nothing to cancel")
        .unwrap();
    assert_eq!(err, Error::UpgradeNotProposed);
}

#[test]
fn migrate_marks_current_version_and_blocks_replay() {
    let ctx = setup(250);

    ctx.client.migrate();
    assert_eq!(
        ctx.client.get_migrated_to_version(),
        Some(String::from_str(&ctx.env, "1.4.0"))
    );

    let err = ctx
        .client
        .try_migrate()
        .err()
        .expect("second migrate rejected")
        .unwrap();
    assert_eq!(err, Error::MigrationAlreadyApplied);
}

// AUTH REGRESSION GUARDS (#73)
//
// setup() mocks all auths for every address, so a call succeeding is not
// proof that require_admin() ran — it succeeds identically whether the
// check is present or was deleted. These tests replace the mock with an
// empty auth set so the call can only succeed if the contract explicitly
// requests and receives the admin's authorization. If require_admin() is
// ever removed from one of these entrypoints, the call stops requesting
// auth altogether and runs to completion instead of failing here,
// turning the test red.

#[test]
fn pause_reverts_without_admin_auth() {
    let ctx = setup(250);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_pause();
    assert!(err.is_err(), "pause must require admin auth");
}

#[test]
fn unpause_reverts_without_admin_auth() {
    let ctx = setup(250);
    ctx.client.pause();
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_unpause();
    assert!(err.is_err(), "unpause must require admin auth");
}

#[test]
fn set_admin_reverts_without_admin_auth() {
    let ctx = setup(250);
    let new_admin = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_admin(&new_admin);
    assert!(err.is_err(), "set_admin must require admin auth");
}

#[test]
fn set_fee_bps_reverts_without_admin_auth() {
    let ctx = setup(250);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_fee_bps(&300);
    assert!(err.is_err(), "set_fee_bps must require admin auth");
}

#[test]
fn set_fee_account_reverts_without_admin_auth() {
    let ctx = setup(250);
    let new_account = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_fee_account(&new_account);
    assert!(err.is_err(), "set_fee_account must require admin auth");
}

#[test]
fn set_profile_contract_reverts_without_admin_auth() {
    let ctx = setup(250);
    let new_profile = Address::generate(&ctx.env);
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_set_profile_contract(&new_profile);
    assert!(err.is_err(), "set_profile_contract must require admin auth");
}

#[test]
fn propose_upgrade_reverts_without_admin_auth() {
    let ctx = setup(250);
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    ctx.env.mock_auths(&[]);
    let err = ctx.client.try_propose_upgrade(&new_hash, &new_version);
    assert!(err.is_err(), "propose_upgrade must require admin auth");
}

#[test]
fn apply_upgrade_reverts_without_admin_auth() {
    let ctx = setup(250);
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
    let ctx = setup(250);
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
    let ctx = setup(250);
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
    let ctx = setup(250);
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
    let ctx = setup(250);
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
