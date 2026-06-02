// boundless-profile: types.
//
// Spec: boundless-credits-reputation-prd.md Section 4.

use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub bootstrapped_at: u64,
    pub credits: u32,
    pub reputation: u64,
    pub wins_count: u32,
    pub submissions_count: u32,
    pub applications_count: u32,
    pub milestones_completed: u32,
}

impl Profile {
    pub fn new(bootstrapped_at: u64, credits: u32) -> Self {
        Self {
            bootstrapped_at,
            credits,
            reputation: 0,
            wins_count: 0,
            submissions_count: 0,
            applications_count: 0,
            milestones_completed: 0,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdmin {
    pub target: Address,
    pub expires_at_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    PendingAdmin,
    EventsContract,
    DefaultBootstrapCredits,
    Paused,
    DeploymentSeq,

    Profile(Address),
    EarningsByToken(Address, Address),

    OpSeen(BytesN<32>),
}
