// boundless-profile: types.
//
// Spec: boundless-credits-reputation-prd.md Section 4.

use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub bootstrapped_at: u64,
    pub credits: u32,
    pub reputation: u64,
}

impl Profile {
    pub fn new(bootstrapped_at: u64, credits: u32) -> Self {
        Self {
            bootstrapped_at,
            credits,
            reputation: 0,
        }
    }
}

// M4 (2026-06 audit): dropped wins_count, submissions_count,
// applications_count, milestones_completed. They were never incremented
// anywhere in the contract — off-chain indexers derive these counters from
// the emitted events instead, which is cheaper and stays accurate without
// a migration if the policy ever changes.

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
    DefaultBootstrapCredits,
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
