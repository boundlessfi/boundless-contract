// boundless-events: admin tests.

#![cfg(test)]

use soroban_sdk::{
    testutils::{BytesN as _, Ledger},
    BytesN, String,
};

use super::common::setup;
use crate::errors::Error;

// Mirror the constants from admin.rs to keep timelock arithmetic readable.
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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "0.2.0"));
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
    // id_base should be (seq << 32); lower 32 bits zero.
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
    // Reuse of InvalidPillar documented in admin.rs.
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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "0.2.0"));
}

#[test]
fn apply_upgrade_after_expiry_reverts() {
    let ctx = setup(250);
    let new_hash: BytesN<32> = BytesN::random(&ctx.env);
    let new_version = String::from_str(&ctx.env, "0.3.0");
    let start = ctx.env.ledger().sequence();
    ctx.client.propose_upgrade(&new_hash, &new_version);

    // Past the expiry.
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
    // Version unchanged.
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "0.2.0"));
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
        Some(String::from_str(&ctx.env, "0.2.0"))
    );

    let err = ctx
        .client
        .try_migrate()
        .err()
        .expect("second migrate rejected")
        .unwrap();
    assert_eq!(err, Error::MigrationAlreadyApplied);
}
