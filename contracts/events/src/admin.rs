use soroban_sdk::{contracttype, panic_with_error, Address, BytesN, Env, Map, String};

use crate::errors::Error;
use crate::events as evt;
use crate::idempotency;
use crate::storage;
use crate::types::{
    DataKey, EventRecord, EventStatus, PendingAdmin, PendingUpgrade, Pillar, ReleaseKind,
};

/// The pre-1.7.0 `EventRecord`, kept only so `migrate` can decode rows written
/// before prize floors replaced the percentage distribution. Nothing else may
/// read or write this shape.
#[contracttype]
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

const PENDING_ADMIN_TTL_LEDGERS: u32 = 120_960;

pub(crate) const MAX_FEE_BPS: u32 = 1_000;

#[cfg(not(feature = "testnet"))]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 17_280;
#[cfg(feature = "testnet")]
const UPGRADE_TIMELOCK_LEDGERS: u32 = 0;
const PENDING_UPGRADE_TTL_LEDGERS: u32 = 518_400;

pub const INITIAL_VERSION: &str = "1.7.0";

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
    let pending = storage::get_pending_admin(env).ok_or(Error::PendingRotationMismatch)?;

    if env.ledger().sequence() > pending.expires_at_ledger {
        storage::clear_pending_admin(env);
        storage::touch_instance(env);
        return Err(Error::PendingRotationExpired);
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
// ============================================================
pub fn propose_upgrade(
    env: &Env,
    new_wasm_hash: BytesN<32>,
    new_version: String,
) -> Result<(), Error> {
    require_admin(env)?;
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
    if current == String::from_str(env, INITIAL_VERSION) {
        migrate_prize_floors(env);
    }

    storage::set_migrated_to_version(env, &current);
    storage::touch_instance(env);
    evt::Migrated {
        from_version,
        to_version: current,
    }
    .publish(env);
    Ok(())
}

/// Rewrites every stored event from the pre-1.7.0 percentage layout to prize
/// floors. `winner_distribution` and `prize_floors` differ in both name and
/// value type, so an old row cannot be decoded by the current struct at all
/// and has to be read through the legacy shape first.
///
/// Percentages were always taken against the escrow balance, so `total_budget *
/// percent / 100` reproduces exactly what each position would have been paid.
///
/// Bounded by the id counter: ids run from `id_base + 1` up to the next id to
/// be issued, and the cap is a backstop against a corrupt counter rather than
/// an expected limit.
fn migrate_prize_floors(env: &Env) {
    const MAX_ROWS: u64 = 256;

    let base = idempotency::id_base(env);
    let next = storage::get_next_event_id(env, base.saturating_add(1));
    let mut id = base.saturating_add(1);
    let mut scanned: u64 = 0;

    while id < next && scanned < MAX_ROWS {
        let key = DataKey::Event(id);
        let legacy: Option<LegacyEventRecord> = env.storage().persistent().get(&key);
        if let Some(old) = legacy {
            let mut floors: Map<u32, i128> = Map::new(env);
            for (position, percent) in old.winner_distribution.iter() {
                let floor = old
                    .total_budget
                    .saturating_mul(percent as i128)
                    .saturating_div(100);
                if floor > 0 {
                    floors.set(position, floor);
                }
            }
            let migrated = EventRecord {
                id: old.id,
                pillar: old.pillar,
                owner: old.owner,
                token: old.token,
                total_budget: old.total_budget,
                remaining_escrow: old.remaining_escrow,
                release_kind: old.release_kind,
                status: old.status,
                content_uri: old.content_uri,
                title: old.title,
                created_at: old.created_at,
                deadline: old.deadline,
                prize_floors: floors,
                fee_bps_override: old.fee_bps_override,
            };
            env.storage().persistent().set(&key, &migrated);
        }
        id = id.saturating_add(1);
        scanned = scanned.saturating_add(1);
    }
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
    storage::touch_instance(env);
    if storage::is_paused(env) {
        return Err(Error::Paused);
    }
    Ok(())
}
