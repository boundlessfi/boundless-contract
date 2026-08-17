#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, Map, String,
};

use super::common::setup;
use crate::errors::Error;
use crate::storage;
use crate::types::{DataKey, EventStatus, Pillar, ReleaseKind, Submission, Winner};

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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.7.0"));
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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.7.0"));
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
    assert_eq!(ctx.client.version(), String::from_str(&ctx.env, "1.7.0"));
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

/// The pre-1.7.0 record layout, written directly into storage so migrate has a
/// legacy row to convert. Mirrors what the two mainnet events look like.
#[soroban_sdk::contracttype]
#[derive(Clone)]
struct LegacyEventRecord {
    pub id: u64,
    pub pillar: Pillar,
    pub owner: Address,
    pub token: Address,
    pub total_budget: i128,
    pub remaining_escrow: i128,
    pub release_kind: ReleaseKind,
    pub status: EventStatus,
    pub content_uri: String,
    pub title: String,
    pub created_at: u64,
    pub deadline: Option<u64>,
    pub winner_distribution: Map<u32, u32>,
    pub fee_bps_override: Option<u32>,
}

#[test]
fn migrate_rewrites_legacy_percentages_as_prize_floors() {
    let ctx = setup(250);
    let budget = 1_000_0000000_i128;

    // Stand in for mainnet event ...610: a 60/40 split, already settled.
    let mut dist = Map::new(&ctx.env);
    dist.set(1, 60_u32);
    dist.set(2, 40_u32);

    let id_base = ctx.client.id_base();
    let event_id = id_base + 1;
    let legacy = LegacyEventRecord {
        id: event_id,
        pillar: Pillar::Bounty,
        owner: Address::generate(&ctx.env),
        token: Address::generate(&ctx.env),
        total_budget: budget,
        remaining_escrow: 0,
        release_kind: ReleaseKind::Single,
        status: EventStatus::Completed,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/legacy"),
        title: String::from_str(&ctx.env, "Muwa Creator Bounty"),
        created_at: 1,
        deadline: None,
        winner_distribution: dist,
        fee_bps_override: None,
    };

    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::Event(event_id), &legacy);
        ctx.env
            .storage()
            .instance()
            .set(&DataKey::NextEventId, &(event_id + 1));
    });

    ctx.client.migrate();

    // Readable again through the current struct, which could not decode it
    // before, and the percentages have become the amounts they always meant.
    let migrated = ctx.client.get_event(&event_id);
    assert_eq!(migrated.prize_floors.get(1), Some(budget * 60 / 100));
    assert_eq!(migrated.prize_floors.get(2), Some(budget * 40 / 100));
    assert_eq!(migrated.total_budget, budget);
    assert_eq!(migrated.status, EventStatus::Completed);
    assert_eq!(
        migrated.title,
        String::from_str(&ctx.env, "Muwa Creator Bounty")
    );
}

#[test]
fn migrate_moves_legacy_submissions_into_slot_zero() {
    let ctx = setup(250);
    let applicant = Address::generate(&ctx.env);
    let event_id = ctx.client.id_base() + 1;

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100_u32);
    let legacy_event = LegacyEventRecord {
        id: event_id,
        pillar: Pillar::Bounty,
        owner: Address::generate(&ctx.env),
        token: Address::generate(&ctx.env),
        total_budget: 1_000_0000000_i128,
        remaining_escrow: 0,
        release_kind: ReleaseKind::Single,
        status: EventStatus::Completed,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/legacy"),
        title: String::from_str(&ctx.env, "Legacy"),
        created_at: 1,
        deadline: None,
        winner_distribution: dist,
        fee_bps_override: None,
    };
    let legacy_submission = Submission {
        applicant: applicant.clone(),
        content_uri: String::from_str(&ctx.env, "ipfs://historical"),
        submitted_at: 42,
    };

    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::Event(event_id), &legacy_event);
        ctx.env
            .storage()
            .instance()
            .set(&DataKey::NextEventId, &(event_id + 1));
        // The applicant index is what bounds the migration scan.
        storage::append_applicant(&ctx.env, event_id, &applicant).unwrap();
        ctx.env.storage().persistent().set(
            &DataKey::EventSubmission(event_id, applicant.clone()),
            &legacy_submission,
        );
    });

    ctx.client.migrate();

    let moved = ctx.client.get_submission(&event_id, &applicant, &0_u32);
    assert_eq!(
        moved.content_uri,
        String::from_str(&ctx.env, "ipfs://historical")
    );
    assert_eq!(moved.submitted_at, 42, "the original timestamp survives");
    assert_eq!(
        ctx.client
            .get_applicant_submission_count(&event_id, &applicant),
        1
    );

    // The old row is gone rather than left as an unreachable duplicate.
    ctx.env.as_contract(&ctx.client.address, || {
        assert!(
            storage::get_legacy_submission(&ctx.env, event_id, &applicant).is_none(),
            "legacy row should be removed once copied"
        );
    });
}

#[test]
fn legacy_submission_is_readable_when_the_applicant_index_never_saw_it() {
    // Hackathon submitters never enter the applicant index, so migrate cannot
    // enumerate them. Their rows must still be reachable as slot 0.
    let ctx = setup(250);
    let submitter = Address::generate(&ctx.env);
    let event_id = ctx.client.id_base() + 1;

    let legacy_submission = Submission {
        applicant: submitter.clone(),
        content_uri: String::from_str(&ctx.env, "ipfs://hackathon-entry"),
        submitted_at: 7,
    };
    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env.storage().persistent().set(
            &DataKey::EventSubmission(event_id, submitter.clone()),
            &legacy_submission,
        );
    });

    let found = ctx.client.get_submission(&event_id, &submitter, &0_u32);
    assert_eq!(
        found.content_uri,
        String::from_str(&ctx.env, "ipfs://hackathon-entry")
    );

    ctx.env.as_contract(&ctx.client.address, || {
        assert!(
            storage::has_any_submission(&ctx.env, event_id, &submitter),
            "an unmigrated row must still block application withdrawal"
        );
    });
}

#[test]
fn migrate_rewrites_zero_amount_grant_winners() {
    // Pre-1.7.0 Multi selections stored amount 0 on the anchor row; leaving
    // that would make every milestone claim revert with nothing to pay.
    let ctx = setup(250);
    let recipient = Address::generate(&ctx.env);
    let event_id = ctx.client.id_base() + 1;
    let budget = 1_000_0000000_i128;

    let mut dist = Map::new(&ctx.env);
    dist.set(1, 100_u32);
    let legacy_event = LegacyEventRecord {
        id: event_id,
        pillar: Pillar::Grant,
        owner: Address::generate(&ctx.env),
        token: Address::generate(&ctx.env),
        total_budget: budget,
        remaining_escrow: budget,
        release_kind: ReleaseKind::Multi(2),
        status: EventStatus::Active,
        content_uri: String::from_str(&ctx.env, "https://api.boundless.fi/grant"),
        title: String::from_str(&ctx.env, "Legacy Grant"),
        created_at: 1,
        deadline: None,
        winner_distribution: dist,
        fee_bps_override: None,
    };

    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env
            .storage()
            .persistent()
            .set(&DataKey::Event(event_id), &legacy_event);
        ctx.env
            .storage()
            .instance()
            .set(&DataKey::NextEventId, &(event_id + 1));
        storage::append_winner(
            &ctx.env,
            event_id,
            &Winner {
                recipient: recipient.clone(),
                position: 1,
                amount: 0,
                milestone: None,
                paid_at: None,
            },
        );
    });

    ctx.client.migrate();

    let rows = ctx.client.get_winners(&event_id);
    assert_eq!(
        rows.get(0).unwrap().amount,
        budget,
        "the anchor must carry what the old percentage would have paid"
    );
}

#[test]
fn migrate_refuses_to_stamp_when_the_id_range_exceeds_the_cap() {
    let ctx = setup(250);
    let base = ctx.client.id_base();
    ctx.env.as_contract(&ctx.client.address, || {
        ctx.env
            .storage()
            .instance()
            .set(&DataKey::NextEventId, &(base + 1_000));
    });

    assert!(
        ctx.client.try_migrate().is_err(),
        "a range beyond the cap must abort rather than half-migrate"
    );
    assert_eq!(
        ctx.client.get_migrated_to_version(),
        None,
        "nothing may be stamped when work would be left undone"
    );
}

#[test]
fn migrate_is_a_no_op_on_a_fresh_deployment() {
    let ctx = setup(250);
    ctx.client.migrate();
    assert_eq!(
        ctx.client.get_migrated_to_version(),
        Some(String::from_str(&ctx.env, "1.7.0"))
    );
}

#[test]
fn migrate_marks_current_version_and_blocks_replay() {
    let ctx = setup(250);

    ctx.client.migrate();
    assert_eq!(
        ctx.client.get_migrated_to_version(),
        Some(String::from_str(&ctx.env, "1.7.0"))
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
