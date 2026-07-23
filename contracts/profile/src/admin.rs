use soroban_sdk::{panic_with_error, Address, BytesN, Env, String};

use crate::errors::Error;
use crate::events as evt;
use crate::storage;
use crate::types::{PendingAdmin, PendingEventsContract, PendingUpgrade};

const PENDING_TTL_LEDGERS: u32 = 120_960;

#[cfg(not(feature = "testnet"))]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
#[cfg(feature = "testnet")]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 0;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

pub const INITIAL_VERSION: &str = "1.2.0";

const EVENTS_CONTRACT_TIMELOCK_LEDGERS: u32 = 17_280;

const PENDING_EVENTS_CONTRACT_TTL_LEDGERS: u32 = 120_960;

pub fn initialize(env: &Env, admin: Address) {
    if env.storage().instance().has(&crate::types::DataKey::Admin) {
        panic_with_error!(env, Error::AlreadyInitialized);
    }

    storage::set_admin(env, &admin);
    storage::set_paused(env, false);
    storage::set_deployment_seq(env, env.ledger().sequence());
    storage::set_version(env, &String::from_str(env, INITIAL_VERSION));
    storage::touch_instance(env);

    evt::AdminUpdated {
        new_admin: admin.clone(),
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

    if now > pending.expires_at_ledger {
        return Err(Error::PendingEventsContractExpired);
    }
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
    if new_version.is_empty() {
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
    // ============================================================

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
    storage::touch_instance(env);
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}
