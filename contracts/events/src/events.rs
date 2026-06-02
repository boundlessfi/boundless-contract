// boundless-events: contract event emissions.
//
// Spec: boundless-platform-contract-prd.md Section 13.
//
// dead_code allowed: event structs are emitted via .publish() calls from
// operation bodies that are stubbed in this pass.
#![allow(dead_code)]

use soroban_sdk::{contractevent, Address, BytesN, String};

use crate::types::Pillar;

#[contractevent]
pub struct EventCreated {
    pub id: u64,
    pub pillar: Pillar,
    pub owner: Address,
    pub token: Address,
    pub total_budget: i128,
    pub content_uri: String,
}

#[contractevent]
pub struct EventCancelled {
    pub id: u64,
}

#[contractevent]
pub struct FundsAdded {
    pub event_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub new_remaining: i128,
}

#[contractevent]
pub struct ContributorRefunded {
    pub event_id: u64,
    pub contributor: Address,
    pub amount: i128,
}

#[contractevent]
pub struct OwnerResidualRefunded {
    pub event_id: u64,
    pub owner: Address,
    pub amount: i128,
}

#[contractevent]
pub struct Applied {
    pub event_id: u64,
    pub applicant: Address,
    pub credit_cost: u32,
}

#[contractevent]
pub struct ApplicationWithdrawn {
    pub event_id: u64,
    pub applicant: Address,
}

#[contractevent]
pub struct Submitted {
    pub event_id: u64,
    pub applicant: Address,
    pub content_uri: String,
}

#[contractevent]
pub struct SubmissionWithdrawn {
    pub event_id: u64,
    pub applicant: Address,
}

#[contractevent]
pub struct WinnersSelected {
    pub event_id: u64,
    pub count: u32,
}

#[contractevent]
pub struct WinnerPaid {
    pub event_id: u64,
    pub recipient: Address,
    pub position: u32,
    pub amount: i128,
    pub milestone: Option<u32>,
}

#[contractevent]
pub struct MilestoneClaimed {
    pub event_id: u64,
    pub recipient: Address,
    pub milestone: u32,
    pub amount: i128,
}

#[contractevent]
pub struct TokenRegistered {
    pub token: Address,
}

#[contractevent]
pub struct TokenDeregistered {
    pub token: Address,
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
pub struct FeeAccountUpdated {
    pub new_account: Address,
}

#[contractevent]
pub struct FeeBpsUpdated {
    pub new_bps: u32,
}

#[contractevent]
pub struct ProfileContractUpdated {
    pub new_addr: Address,
}

#[contractevent]
pub struct Paused {}

#[contractevent]
pub struct Unpaused {}

#[contractevent]
pub struct Upgraded {
    pub new_wasm_hash: BytesN<32>,
}

// Linker keep-alive: a synthetic constant referenced from lib.rs so that the
// module participates in the binary even if some events have no callers yet.
pub const EVENTS_LINK_KEEP: u32 = 0;
