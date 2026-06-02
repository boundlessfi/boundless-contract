// boundless-events: admin operations.
//
// Spec: boundless-platform-contract-prd.md Section 6.1.

use soroban_sdk::{panic_with_error, Address, BytesN, Env};

use crate::errors::Error;
use crate::events as evt;
use crate::storage;
use crate::types::PendingAdmin;

// Two-step admin rotation TTL: 7 days at the testnet 5-second ledger cadence.
// 7 * 24 * 60 * 60 / 5 = 120,960 ledgers.
const PENDING_ADMIN_TTL_LEDGERS: u32 = 120_960;

// Fee bps cap. 100% = 10_000 bps; we cap below 100% as a sanity bound.
const MAX_FEE_BPS: u32 = 5_000;

// ============================================================
// INITIALIZATION
// ============================================================
pub fn initialize(
    env: &Env,
    admin: Address,
    fee_account: Address,
    fee_bps: u32,
    profile_contract: Address,
) {
    // Refuse double-init by checking the admin key.
    if env
        .storage()
        .persistent()
        .has(&crate::types::DataKey::Admin)
    {
        panic_with_error!(env, Error::AlreadyInitialized);
    }
    if fee_bps > MAX_FEE_BPS {
        panic_with_error!(env, Error::InvalidFeeBps);
    }

    storage::set_admin(env, &admin);
    storage::set_fee_account(env, &fee_account);
    storage::set_fee_bps(env, fee_bps);
    storage::set_profile_contract(env, &profile_contract);
    storage::set_deployment_seq(env, env.ledger().sequence());
    storage::set_paused(env, false);

    evt::AdminUpdated {
        new_admin: admin.clone(),
    }
    .publish(env);
    evt::FeeAccountUpdated {
        new_account: fee_account,
    }
    .publish(env);
    evt::FeeBpsUpdated { new_bps: fee_bps }.publish(env);
    evt::ProfileContractUpdated {
        new_addr: profile_contract,
    }
    .publish(env);
}

// ============================================================
// ADMIN ROTATION (two-step)
// ============================================================
pub fn set_admin(env: &Env, new_admin: Address) -> Result<(), Error> {
    require_admin(env)?;

    let expires_at = env
        .ledger()
        .sequence()
        .saturating_add(PENDING_ADMIN_TTL_LEDGERS);
    let pending = PendingAdmin {
        target: new_admin.clone(),
        expires_at_ledger: expires_at,
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
// FEE CONFIG
// ============================================================
pub fn set_fee_bps(env: &Env, new_bps: u32) -> Result<(), Error> {
    require_admin(env)?;
    if new_bps > MAX_FEE_BPS {
        return Err(Error::InvalidFeeBps);
    }
    storage::set_fee_bps(env, new_bps);
    evt::FeeBpsUpdated { new_bps }.publish(env);
    Ok(())
}

pub fn set_fee_account(env: &Env, new_account: Address) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_fee_account(env, &new_account);
    evt::FeeAccountUpdated {
        new_account: new_account.clone(),
    }
    .publish(env);
    Ok(())
}

// ============================================================
// PROFILE CONTRACT BINDING
// ============================================================
pub fn set_profile_contract(env: &Env, new_addr: Address) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_profile_contract(env, &new_addr);
    evt::ProfileContractUpdated {
        new_addr: new_addr.clone(),
    }
    .publish(env);
    Ok(())
}

// ============================================================
// PAUSE
// ============================================================
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

// ============================================================
// UPGRADE
// ============================================================
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

pub fn get_fee_bps(env: &Env) -> u32 {
    storage::get_fee_bps(env)
}

pub fn get_fee_account(env: &Env) -> Address {
    storage::get_fee_account(env)
}

pub fn get_profile_contract(env: &Env) -> Address {
    storage::get_profile_contract(env)
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

pub fn require_not_paused(env: &Env) -> Result<(), Error> {
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}

