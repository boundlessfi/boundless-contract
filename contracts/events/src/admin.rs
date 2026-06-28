// boundless-events: admin operations.
//
// Spec: boundless-platform-contract-prd.md Section 6.1.

use soroban_sdk::{panic_with_error, Address, BytesN, Env, String};

use crate::errors::Error;
use crate::events as evt;
use crate::storage;
use crate::types::{PendingAdmin, PendingUpgrade};

// Two-step admin rotation TTL: 7 days at the mainnet 5-second ledger cadence.
// 7 * 24 * 60 * 60 / 5 = 120_960 ledgers.
const PENDING_ADMIN_TTL_LEDGERS: u32 = 120_960;

// Fee bps cap. 100% = 10_000 bps. L4 (2026-06 audit): tightened from 5_000
// (50%) to 1_000 (10%). 10% covers the full envelope of real Boundless
// pricing tiers; a config typo can no longer push the fee above operating
// range. Per-event overrides still respect this cap.
pub(crate) const MAX_FEE_BPS: u32 = 1_000;

// H6: timelocked upgrade windows.
//
//   UPGRADE_TIMELOCK_LEDGERS    earliest gap between propose and apply.
//                                ~1 day so off-chain monitors have a window
//                                to react before the new wasm lands.
//   PENDING_UPGRADE_TTL_LEDGERS  hard expiry on the proposal; ~30 days.
//                                Past this the admin must re-propose.
// Testnet builds (`--features testnet`) zero the upgrade timelock for fast
// iteration; the default build (mainnet + everything else) keeps the full
// ~1-day timelock. Fail-safe: omitting the flag yields the secure value, never 0.
#[cfg(not(feature = "testnet"))]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
#[cfg(feature = "testnet")]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 0;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

// Initial contract version. Written by __constructor and bumped on
// apply_upgrade. Bump alongside any storage-layout or public-surface change
// that warrants a migration entrypoint.
pub const INITIAL_VERSION: &str = "1.0.0";

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
    // Refuse double-init by checking the admin key in instance storage (the
    // new home for admin/config per the 2026-06 audit).
    if env.storage().instance().has(&crate::types::DataKey::Admin) {
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
    storage::set_version(env, &String::from_str(env, INITIAL_VERSION));
    storage::touch_instance(env);

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
// FEE CONFIG
// ============================================================
pub fn set_fee_bps(env: &Env, new_bps: u32) -> Result<(), Error> {
    require_admin(env)?;
    if new_bps > MAX_FEE_BPS {
        return Err(Error::InvalidFeeBps);
    }
    storage::set_fee_bps(env, new_bps);
    storage::touch_instance(env);
    evt::FeeBpsUpdated { new_bps }.publish(env);
    Ok(())
}

pub fn set_fee_account(env: &Env, new_account: Address) -> Result<(), Error> {
    require_admin(env)?;
    // M2 (2026-06 audit): we do not verify trustline existence at the
    // contract layer because Soroban's SAC interface cannot reliably
    // distinguish "no trustline" from "zero balance". Admin must verify
    // off-chain BEFORE calling this; the FeeAccountUpdated event below is
    // the signal off-chain monitors rely on to re-verify. See
    // docs/audit-2026-06-stellar-skill.md M2.
    storage::set_fee_account(env, &new_account);
    storage::touch_instance(env);
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
    storage::touch_instance(env);
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
//
// Three steps:
//   1. propose_upgrade(wasm_hash, new_version) — admin-only; writes
//      PendingUpgrade with proposed_at = now, available_at = now + TIMELOCK,
//      expires_at = now + TTL. Off-chain monitors can see exactly which
//      version + wasm is queued before it lands.
//   2. apply_upgrade()                          — admin-only; requires
//      now in [available_at, expires_at]; swaps the wasm hash and bumps
//      the on-chain version label.
//   3. cancel_pending_upgrade()                 — admin-only; prunes a stale
//      or unwanted proposal so a fresh one can be queued.
//
// migrate(to_version) is a SEPARATE call that runs the one-shot data
// migration matched to the just-applied version. Guard via MigratedToVersion.
//
// Spec: docs/audit-2026-06-stellar-skill.md H6.
// ============================================================
pub fn propose_upgrade(
    env: &Env,
    new_wasm_hash: BytesN<32>,
    new_version: String,
) -> Result<(), Error> {
    require_admin(env)?;
    // Empty version is rejected; reuse InvalidPillar to stay inside the
    // soroban contracterror 50-variant cap (a dedicated InvalidVersion
    // would push us over). Off-chain monitors should treat InvalidPillar
    // on propose_upgrade as "bad version label."
    if new_version.is_empty() {
        return Err(Error::InvalidPillar);
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
    // Keep the legacy Upgraded event for indexers built against the old shape.
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
// Called once per version after apply_upgrade swaps the wasm. The shape
// is:
//
//   1. Read the current Version label (set by apply_upgrade) and the
//      previously-applied migration marker (MigratedToVersion). If the
//      marker already equals the current Version, reject as
//      MigrationAlreadyApplied — a second invocation is always a
//      misconfiguration.
//   2. Dispatch on (prev, current) and run the migration body. Bodies
//      run cleanly inside the same tx as the marker write, so a failure
//      reverts both — there is no half-migrated state to recover from.
//   3. Stamp MigratedToVersion = current and emit Migrated{}.
//
// Mainnet bootstrap: the first deploy lands the constructor with the
// current storage layout, so no migration body is needed. The first real
// migration body will land with the first storage-layout upgrade after
// mainnet goes live. We keep an empty match arm for the no-op case so the
// shape is stable and future contributors do not have to debate where
// the dispatch goes.
//
// NB: Soroban String only supports equality + length, no `as_str()` /
// pattern matching. The dispatch below uses `String::from_str` + equality.
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
    // Each future upgrade adds an `if` clause here with its migration body.
    // Touch only persistent / instance entries that the new layout changes;
    // anything the new code reads with backwards-compatible defaults can
    // be left alone.
    //
    // Pattern:
    //
    //     if from_version == String::from_str(env, "0.2.0")
    //         && current == String::from_str(env, "0.3.0")
    //     {
    //         migrate_0_2_0_to_0_3_0(env)?;
    //     }
    //
    // The corresponding private fn lives below the match block. Keep it
    // small enough to read; if the migration is large, split it into named
    // helpers and call from inside the body.
    // ============================================================

    // No-op for the initial 0.2.0 deploy. __constructor populates storage
    // in the current shape, so admin can call migrate() once just to stamp
    // the marker and unlock the audit trail (the Migrated event signals
    // off-chain runbooks that the post-upgrade cleanup ran).

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

pub fn require_not_paused(env: &Env) -> Result<(), Error> {
    // Every operation path runs this first, so this is the single spot to
    // bump instance TTL on the hot path. Admin paths bump explicitly.
    storage::touch_instance(env);
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}
