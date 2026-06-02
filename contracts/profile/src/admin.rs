// boundless-profile: admin operations.
//
// Two-step rotations for both admin and events_contract.
//
// Spec: boundless-credits-reputation-prd.md Sections 5.5, 6.

use soroban_sdk::{panic_with_error, Address, BytesN, Env};

use crate::errors::Error;
use crate::events as evt;
use crate::storage;
use crate::types::PendingAdmin;

const PENDING_TTL_LEDGERS: u32 = 120_960; // 7 days at ~5 sec per ledger.

pub fn initialize(env: &Env, admin: Address, default_bootstrap_credits: u32) {
    if env
        .storage()
        .persistent()
        .has(&crate::types::DataKey::Admin)
    {
        panic_with_error!(env, Error::AlreadyInitialized);
    }

    storage::set_admin(env, &admin);
    storage::set_default_bootstrap_credits(env, default_bootstrap_credits);
    storage::set_paused(env, false);
    storage::set_deployment_seq(env, env.ledger().sequence());

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
    evt::PendingAdminSet { target: new_admin }.publish(env);
    Ok(())
}

pub fn accept_admin(env: &Env) -> Result<(), Error> {
    let pending = storage::get_pending_admin(env).ok_or(Error::PendingAdminMismatch)?;
    if env.ledger().sequence() > pending.expires_at_ledger {
        storage::clear_pending_admin(env);
        return Err(Error::PendingAdminExpired);
    }
    pending.target.require_auth();
    storage::set_admin(env, &pending.target);
    storage::clear_pending_admin(env);
    evt::AdminUpdated {
        new_admin: pending.target,
    }
    .publish(env);
    Ok(())
}

// ============================================================
// EVENTS CONTRACT BINDING (single-step; admin multisig is the protection)
// ============================================================
pub fn set_events_contract(env: &Env, new_addr: Address) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_events_contract(env, &new_addr);
    evt::EventsContractUpdated {
        new_addr: new_addr.clone(),
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
    evt::BootstrapAmountSet { new_amount }.publish(env);
    Ok(())
}

pub fn pause(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_paused(env, true);
    evt::Paused {}.publish(env);
    Ok(())
}

pub fn unpause(env: &Env) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_paused(env, false);
    evt::Unpaused {}.publish(env);
    Ok(())
}

pub fn upgrade(env: &Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
    require_admin(env)?;
    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());
    evt::Upgraded { new_wasm_hash }.publish(env);
    Ok(())
}

// ============================================================
// READS
// ============================================================
pub fn get_admin(env: &Env) -> Address {
    storage::get_admin(env).expect("admin not configured")
}

pub fn get_events_contract(env: &Env) -> Option<Address> {
    storage::get_events_contract(env)
}

pub fn get_default_bootstrap_credits(env: &Env) -> u32 {
    storage::get_default_bootstrap_credits(env)
}

pub fn is_paused(env: &Env) -> bool {
    storage::is_paused(env)
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
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}

