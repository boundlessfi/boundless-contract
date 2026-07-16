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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdmin {
    pub target: Address,
    pub expires_at_ledger: u32,
}

// ============================================================
// PENDING EVENTS CONTRACT (two-step rotation w/ timelock)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingEventsContract {
    pub target: Address,
    pub proposed_at_ledger: u32,
    pub expires_at_ledger: u32,
}

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

    Version,
    PendingUpgrade,
    MigratedToVersion,

    OpSeen(BytesN<32>),
}
