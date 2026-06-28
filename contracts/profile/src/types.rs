// boundless-profile: types.
//
// Spec: boundless-credits-reputation-prd.md Section 4.

use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub bootstrapped_at: u64,
    pub reputation: u64,
}

impl Profile {
    pub fn new(bootstrapped_at: u64) -> Self {
        Self {
            bootstrapped_at,
            reputation: 0,
        }
    }
}

// M4 (2026-06 audit): dropped wins_count, submissions_count,
// applications_count, milestones_completed. They were never incremented
// anywhere in the contract.
//
// 2026-06: dropped `credits`. Credits are now an off-chain ledger
// (boundless-nestjs); the profile contract holds only reputation + earnings.
// Removing the field changes the persisted Profile layout: existing on-chain
// profiles must be re-bootstrapped or migrated before this version is applied.
// See the mainnet-deploy-runbook before upgrading.

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdmin {
    pub target: Address,
    pub expires_at_ledger: u32,
}

// ============================================================
// PENDING EVENTS CONTRACT (two-step rotation w/ timelock)
//
// proposed_at_ledger gates the early-finalize window; accept can fire only
// after proposed_at_ledger + EVENTS_CONTRACT_TIMELOCK_LEDGERS. expires_at_ledger
// gates the late-finalize window; after expiry the proposal must be re-issued.
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingEventsContract {
    pub target: Address,
    pub proposed_at_ledger: u32,
    pub expires_at_ledger: u32,
}

// H6: timelocked wasm rotation. Mirrors the events contract's PendingUpgrade.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub new_version: String,
    pub proposed_at_ledger: u32,
    pub available_at_ledger: u32,
    pub expires_at_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    PendingAdmin,
    EventsContract,
    PendingEventsContract,
    Paused,
    DeploymentSeq,

    Profile(Address),
    EarningsByToken(Address, Address),

    // H6: contract semver, timelocked upgrade slot, last migrated-to version.
    Version,
    PendingUpgrade,
    MigratedToVersion,

    OpSeen(BytesN<32>),
}
