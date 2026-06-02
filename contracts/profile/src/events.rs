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
    pub initial_credits: u32,
}

#[contractevent]
pub struct CreditsSpent {
    pub user: Address,
    pub amount: u32,
    pub reason: Symbol,
}

#[contractevent]
pub struct CreditsEarned {
    pub user: Address,
    pub amount: u32,
    pub reason: Symbol,
}

#[contractevent]
pub struct CreditsRefunded {
    pub user: Address,
    pub amount: u32,
    pub reason: Symbol,
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
pub struct AdminCreditsGranted {
    pub user: Address,
    pub amount: u32,
    pub reason: String,
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
pub struct BootstrapAmountSet {
    pub new_amount: u32,
}

#[contractevent]
pub struct Paused {}

#[contractevent]
pub struct Unpaused {}

#[contractevent]
pub struct Upgraded {
    pub new_wasm_hash: BytesN<32>,
}
