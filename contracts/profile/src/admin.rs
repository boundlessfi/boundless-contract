// boundless-profile: admin operations.
//
// Two-step rotations for both admin and events_contract.
//
// Spec: boundless-credits-reputation-prd.md Sections 5.5, 6.

use soroban_sdk::{panic_with_error, Address, BytesN, Env, String};

use crate::errors::Error;
use crate::events as evt;
use crate::storage;
use crate::types::{PendingAdmin, PendingEventsContract, PendingUpgrade};

const PENDING_TTL_LEDGERS: u32 = 120_960; // 7 days at ~5 sec per ledger.

// H6: timelocked upgrade windows. Match the events contract for consistency.
const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

pub const INITIAL_VERSION: &str = "0.2.0";

// Events-contract rotation timelock: minimum delay between propose and
// accept so off-chain monitoring has a window to react to a malicious
// proposal. ~1 day at 5 sec per ledger.
//
// Spec: docs/audit-2026-06-stellar-skill.md, H5.
const EVENTS_CONTRACT_TIMELOCK_LEDGERS: u32 = 17_280;

// Maximum window between propose and accept. After this the proposal must
// be re-issued. Matches PENDING_TTL_LEDGERS for symmetry with admin rotation.
const PENDING_EVENTS_CONTRACT_TTL_LEDGERS: u32 = 120_960;

pub fn initialize(env: &Env, admin: Address, default_bootstrap_credits: u32) {
    if env.storage().instance().has(&crate::types::DataKey::Admin) {
        panic_with_error!(env, Error::AlreadyInitialized);
    }

    storage::set_admin(env, &admin);
    storage::set_default_bootstrap_credits(env, default_bootstrap_credits);
    storage::set_paused(env, false);
    storage::set_deployment_seq(env, env.ledger().sequence());
    storage::set_version(env, &String::from_str(env, INITIAL_VERSION));
    storage::touch_instance(env);

    evt::AdminUpdated {
        new_admin: admin.clone(),
    }
    .publish(env);
    evt::BootstrapAmountSet {
        new_amount: default_bootstrap_credits,
    }
    .publish(env);
}

// ============================================================
// ADMIN ROTATION
// ============================================================
pub fn set_admin(env: &Env, new_admin: Address) -> Result<(), Error> {
    require_admin(env)?;
    let pending = PendingAdmin {
        target: new_admin.clone(),
        expires_at_ledger: env.ledger().sequence().saturating_add(PENDING_TTL_LEDGERS),
    };
    storage::set_pending_admin(env, &pending);
    storage::touch_instance(env);
    evt::PendingAdminSet { target: new_admin }.publish(env);
    Ok(())
}

pub fn accept_admin(env: &Env) -> Result<(), Error> {
    let pending = storage::get_pending_admin(env).ok_or(Error::PendingAdminMismatch)?;
    if env.ledger().sequence() > pending.expires_at_ledger {
        storage::clear_pending_admin(env);
        storage::touch_instance(env);
        return Err(Error::PendingAdminExpired);
    }
    pending.target.require_auth();
    storage::set_admin(env, &pending.target);
    storage::clear_pending_admin(env);
    storage::touch_instance(env);
    evt::AdminUpdated {
        new_admin: pending.target,
    }
    .publish(env);
    Ok(())
}

// ============================================================
// EVENTS CONTRACT BINDING
//
// First-set is single-step (deploy bootstrap; there's no live contract to
// rotate from). Subsequent rotations require propose + accept with a
// timelock window so off-chain monitors have time to react to a malicious
// or mistaken proposal before it lands. Closes audit finding H5 (the prior
// single-step rotation was the single soft point in the auth chain for
// every credit/reputation/earnings mutation).
//
// Spec: docs/audit-2026-06-stellar-skill.md, H5.
// ============================================================
pub fn set_events_contract(env: &Env, new_addr: Address) -> Result<(), Error> {
    require_admin(env)?;
    if storage::get_events_contract(env).is_some() {
        return Err(Error::EventsContractAlreadyConfigured);
    }
    storage::set_events_contract(env, &new_addr);
    storage::touch_instance(env);
    evt::EventsContractUpdated {
        new_addr: new_addr.clone(),
    }
    .publish(env);
    Ok(())
}

pub fn propose_events_contract(env: &Env, new_addr: Address) -> Result<(), Error> {
    require_admin(env)?;
    let proposed_at = env.ledger().sequence();
    let expires_at = proposed_at.saturating_add(PENDING_EVENTS_CONTRACT_TTL_LEDGERS);
    let pending = PendingEventsContract {
        target: new_addr.clone(),
        proposed_at_ledger: proposed_at,
        expires_at_ledger: expires_at,
    };
    storage::set_pending_events_contract(env, &pending);
    storage::touch_instance(env);
    evt::PendingEventsContractSet {
        target: new_addr,
        proposed_at_ledger: proposed_at,
        expires_at_ledger: expires_at,
    }
    .publish(env);
    Ok(())
}

pub fn accept_events_contract(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    let pending =
        storage::get_pending_events_contract(env).ok_or(Error::PendingEventsContractMismatch)?;
    let now = env.ledger().sequence();

    // Late finalize: proposal expired. We do NOT clear here because the
    // Err return reverts every storage write in this tx anyway; the expired
    // entry stays put and admin must call cancel_pending_events_contract to
    // prune it before re-proposing.
    if now > pending.expires_at_ledger {
        return Err(Error::PendingEventsContractExpired);
    }
    // Early finalize: still inside the timelock window.
    let earliest = pending
        .proposed_at_ledger
        .saturating_add(EVENTS_CONTRACT_TIMELOCK_LEDGERS);
    if now < earliest {
        return Err(Error::PendingEventsContractTimelock);
    }

    storage::set_events_contract(env, &pending.target);
    storage::clear_pending_events_contract(env);
    storage::touch_instance(env);
    evt::EventsContractUpdated {
        new_addr: pending.target,
    }
    .publish(env);
    Ok(())
}

pub fn cancel_pending_events_contract(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    if storage::get_pending_events_contract(env).is_none() {
        return Err(Error::PendingEventsContractMismatch);
    }
    storage::clear_pending_events_contract(env);
    storage::touch_instance(env);
    evt::EventsRotationCancelled {
        cancelled_at_ledger: env.ledger().sequence(),
    }
    .publish(env);
    Ok(())
}

// ============================================================
// CONFIG
// ============================================================
pub fn set_default_bootstrap_credits(env: &Env, new_amount: u32) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_default_bootstrap_credits(env, new_amount);
    storage::touch_instance(env);
    evt::BootstrapAmountSet { new_amount }.publish(env);
    Ok(())
}

pub fn pause(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_paused(env, true);
    storage::touch_instance(env);
    evt::Paused {}.publish(env);
    Ok(())
}

pub fn unpause(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_paused(env, false);
    storage::touch_instance(env);
    evt::Unpaused {}.publish(env);
    Ok(())
}

// ============================================================
// UPGRADE (timelocked; H6)
// ============================================================
pub fn propose_upgrade(
    env: &Env,
    new_wasm_hash: BytesN<32>,
    new_version: String,
) -> Result<(), Error> {
    require_admin(env)?;
    if new_version.len() == 0 {
        // Reuse existing AdminCannotBeZero semantic for "empty input".
        return Err(Error::AdminCannotBeZero);
    }
    let now = env.ledger().sequence();
    let available_at = now.saturating_add(UPGRADE_TIMELOCK_LEDGERS);
    let expires_at = now.saturating_add(PENDING_UPGRADE_TTL_LEDGERS);
    let pending = PendingUpgrade {
        wasm_hash: new_wasm_hash.clone(),
        new_version: new_version.clone(),
        proposed_at_ledger: now,
        available_at_ledger: available_at,
        expires_at_ledger: expires_at,
    };
    storage::set_pending_upgrade(env, &pending);
    storage::touch_instance(env);
    evt::PendingUpgradeProposed {
        wasm_hash: new_wasm_hash,
        new_version,
        available_at_ledger: available_at,
        expires_at_ledger: expires_at,
    }
    .publish(env);
    Ok(())
}

pub fn apply_upgrade(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    let pending = storage::get_pending_upgrade(env).ok_or(Error::UpgradeNotProposed)?;
    let now = env.ledger().sequence();
    if now > pending.expires_at_ledger {
        return Err(Error::UpgradeProposalExpired);
    }
    if now < pending.available_at_ledger {
        return Err(Error::UpgradeTimelockNotElapsed);
    }
    storage::touch_instance(env);
    env.deployer()
        .update_current_contract_wasm(pending.wasm_hash.clone());
    storage::set_version(env, &pending.new_version);
    storage::clear_pending_upgrade(env);
    evt::UpgradeApplied {
        wasm_hash: pending.wasm_hash.clone(),
        new_version: pending.new_version.clone(),
    }
    .publish(env);
    evt::Upgraded {
        new_wasm_hash: pending.wasm_hash,
    }
    .publish(env);
    Ok(())
}

pub fn cancel_pending_upgrade(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    if storage::get_pending_upgrade(env).is_none() {
        return Err(Error::UpgradeNotProposed);
    }
    storage::clear_pending_upgrade(env);
    storage::touch_instance(env);
    evt::PendingUpgradeCancelled {
        cancelled_at_ledger: env.ledger().sequence(),
    }
    .publish(env);
    Ok(())
}

// ============================================================
// MIGRATE (post-upgrade one-shot; H6)
//
// Mirror of the events contract's migrate(). See contracts/events/src/admin.rs
// for the full pattern + dispatch-block convention. The profile contract has
// a simpler storage layout, so most upgrades will not need a migration body
// here; the empty pass-through still stamps MigratedToVersion so off-chain
// runbooks see a Migrated event.
// ============================================================
pub fn migrate(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    let current = storage::get_version(env).ok_or(Error::NotInitialized)?;
    let already = storage::get_migrated_to_version(env);
    if let Some(m) = already.clone() {
        if m == current {
            return Err(Error::MigrationAlreadyApplied);
        }
    }

    let from_version = already.unwrap_or_else(|| String::from_str(env, "0.0.0"));

    // ============================================================
    // PER-(from -> to) MIGRATION DISPATCH
    //
    //     if from_version == String::from_str(env, "0.2.0")
    //         && current == String::from_str(env, "0.3.0")
    //     {
    //         migrate_0_2_0_to_0_3_0(env)?;
    //     }
    //
    // Soroban String only supports equality + length, so dispatch is via
    // `String::from_str` + `==`. Keep bodies inline unless the migration
    // grows past ~30 lines, then promote into a private fn below.
    // ============================================================

    // No-op for the initial 0.2.0 deploy. __constructor populates storage
    // in the current shape; admin still calls migrate() once after deploy
    // so the audit trail records that the post-upgrade cleanup ran.

    storage::set_migrated_to_version(env, &current);
    storage::touch_instance(env);
    evt::Migrated {
        from_version,
        to_version: current,
    }
    .publish(env);
    Ok(())
}

// ============================================================
// READS
// ============================================================
pub fn get_admin(env: &Env) -> Address {
    storage::get_admin(env).unwrap_or_else(|_| panic_with_error!(env, Error::NotInitialized))
}

pub fn get_events_contract(env: &Env) -> Option<Address> {
    storage::get_events_contract(env)
}

pub fn get_pending_events_contract(env: &Env) -> Option<PendingEventsContract> {
    storage::get_pending_events_contract(env)
}

pub fn get_default_bootstrap_credits(env: &Env) -> u32 {
    storage::get_default_bootstrap_credits(env)
}

pub fn is_paused(env: &Env) -> bool {
    storage::is_paused(env)
}

pub fn get_version(env: &Env) -> String {
    storage::get_version(env).unwrap_or_else(|| panic_with_error!(env, Error::NotInitialized))
}

pub fn get_pending_upgrade(env: &Env) -> Option<PendingUpgrade> {
    storage::get_pending_upgrade(env)
}

pub fn get_migrated_to_version(env: &Env) -> Option<String> {
    storage::get_migrated_to_version(env)
}

// ============================================================
// AUTH GUARDS
// ============================================================
pub fn require_admin(env: &Env) -> Result<(), Error> {
    let admin = storage::get_admin(env)?;
    admin.require_auth();
    Ok(())
}

pub fn require_events_contract(env: &Env) -> Result<(), Error> {
    let events = storage::get_events_contract(env).ok_or(Error::EventsContractNotConfigured)?;
    events.require_auth();
    Ok(())
}

pub fn require_not_paused(env: &Env) -> Result<(), Error> {
    // Every state-mutating operation runs this first; single touchpoint for
    // bumping instance TTL on the hot path. Admin paths bump explicitly.
    storage::touch_instance(env);
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}
