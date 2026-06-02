// boundless-events: on-chain types.
//
// Spec: boundless-platform-contract-prd.md Sections 5.1 to 5.2.

use soroban_sdk::{contracttype, Address, BytesN, Map, String};

// ============================================================
// PILLAR
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pillar {
    Hackathon,
    Bounty,
    Grant,
}

// ============================================================
// STATUS
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventStatus {
    Active,
    Cancelled,
    Completed,
}

// ============================================================
// RELEASE KIND
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseKind {
    Single,
    Multi(u32),
}

// ============================================================
// EVENT RECORD
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRecord {
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
    pub application_credit_cost: u32,
}

// ============================================================
// CREATE-EVENT PARAMS (packed to stay under Soroban's 10-param fn limit)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateEventParams {
    pub pillar: Pillar,
    pub owner: Address,
    pub token: Address,
    pub total_budget: i128,
    pub release_kind: ReleaseKind,
    pub content_uri: String,
    pub title: String,
    pub deadline: Option<u64>,
    pub winner_distribution: Map<u32, u32>,
    pub application_credit_cost: u32,
}

// ============================================================
// SUBMISSION
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submission {
    pub applicant: Address,
    pub content_uri: String,
    pub submitted_at: u64,
}

// ============================================================
// CONTRIBUTION
//
// Partner / community top-up to an event's escrow. Owner deposits via
// create_event are NOT recorded as Contribution entries; they live in
// event.owner / event.total_budget. Contributions are recorded so that
// cancel_event refunds partners ahead of the owner per the policy in
// boundless-partner-contributions-prd.md.
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contribution {
    pub contributor: Address,
    pub amount: i128,
    pub contributed_at: u64,
}

// ============================================================
// WINNER
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Winner {
    pub recipient: Address,
    pub position: u32,
    pub amount: i128,
    pub milestone: Option<u32>,
    pub paid_at: Option<u64>,
}

// ============================================================
// WINNER SELECTION SPEC
//
// Input to select_winners. The orchestrator computes credit_earn and
// reputation_bump off-chain per the policy tables in
// boundless-credits-reputation-prd.md Section 9; the contract just records.
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinnerSpec {
    pub recipient: Address,
    pub position: u32,
    pub credit_earn: u32,
    pub reputation_bump: u32,
}

// ============================================================
// STORAGE DATA KEYS
// ============================================================
#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    // Admin / config
    Admin,
    PendingAdmin,
    FeeAccount,
    FeeBps,
    Paused,
    DeploymentSeq,
    ProfileContract,

    // Token whitelist
    SupportedToken(Address),

    // Events
    NextEventId,
    Event(u64),
    EventApplicants(u64),
    EventSubmissions(u64),
    EventWinners(u64),

    // Partner contributions to an event's escrow.
    // ContributorAmount holds the running total per (event_id, contributor).
    // ContributorList preserves the deposit order so refunds and reads can
    // iterate without scanning the full map. Owner deposits do NOT appear
    // in either key; they're tracked via event.owner / event.total_budget.
    ContributorAmount(u64, Address),
    ContributorList(u64),

    // Grant milestone tracking: (event_id, recipient, milestone) -> bool
    MilestoneClaimed(u64, Address, u32),

    // Idempotency
    OpSeen(BytesN<32>),
}

// ============================================================
// PENDING ADMIN payload (target + expiry ledger)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdmin {
    pub target: Address,
    pub expires_at_ledger: u32,
}
