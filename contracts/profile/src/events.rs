// boundless-profile: contract event emissions.
//
// Spec: boundless-credits-reputation-prd.md Section 7.
//
// dead_code allowed: event structs are emitted via .publish() calls. Some
// events are emitted only by ops still wiring up.
#![allow(dead_code)]

use soroban_sdk::{contractevent, Address, BytesN, String, Symbol};

#[contractevent]
pub struct ProfileBootstrapped {
    pub user: Address,
}

#[contractevent]
pub struct ReputationBumped {
    pub user: Address,
    pub delta: u32,
    pub reason: Symbol,
}

#[contractevent]
pub struct ReputationSlashed {
    pub user: Address,
    pub delta: u32,
    pub reason: Symbol,
}

#[contractevent]
pub struct EarningsRegistered {
    pub user: Address,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
pub struct AdminReputationSlashed {
    pub user: Address,
    pub delta: u32,
    pub reason: String,
}

#[contractevent]
pub struct AdminUpdated {
    pub new_admin: Address,
}

#[contractevent]
pub struct PendingAdminSet {
    pub target: Address,
}

#[contractevent]
pub struct EventsContractUpdated {
    pub new_addr: Address,
}

#[contractevent]
pub struct PendingEventsContractSet {
    pub target: Address,
    pub proposed_at_ledger: u32,
    pub expires_at_ledger: u32,
}

#[contractevent]
pub struct EventsRotationCancelled {
    pub cancelled_at_ledger: u32,
}

#[contractevent]
pub struct Paused {}

#[contractevent]
pub struct Unpaused {}

#[contractevent]
pub struct Upgraded {
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent]
pub struct PendingUpgradeProposed {
    pub wasm_hash: BytesN<32>,
    pub new_version: String,
    pub available_at_ledger: u32,
    pub expires_at_ledger: u32,
}

#[contractevent]
pub struct PendingUpgradeCancelled {
    pub cancelled_at_ledger: u32,
}

#[contractevent]
pub struct UpgradeApplied {
    pub wasm_hash: BytesN<32>,
    pub new_version: String,
}

#[contractevent]
pub struct Migrated {
    pub from_version: String,
    pub to_version: String,
}
