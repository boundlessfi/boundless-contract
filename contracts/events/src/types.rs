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
    Crowdfunding,
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
    Cancelling,
}

// ============================================================
// CANCELLATION
// ============================================================
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationBranch {
    OwnerOnly,
    FullPartnerThenResidual,
    ProRataPartners,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancellationState {
    pub non_owner_total: i128,
    pub remaining_at_start: i128,
    pub count_at_start: u32,
    pub next_idx: u32,
    pub branch: CancellationBranch,
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
    pub fee_bps_override: Option<u32>,
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
    pub fee_bps_override: Option<u32>,
    pub manager: Option<Address>,
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
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinnerSpec {
    pub recipient: Address,
    pub position: u32,
    pub reputation_bump: u32,
}

// ============================================================
// STORAGE DATA KEYS
// ============================================================
#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    PendingAdmin,
    FeeAccount,
    FeeBps,
    Paused,
    DeploymentSeq,
    ProfileContract,

    SupportedToken(Address),

    NextEventId,
    Event(u64),

    EventManager(u64),

    EventApplicantCount(u64),
    EventApplicantAt(u64, u32),
    EventApplicantSlot(u64, Address),

    EventSubmission(u64, Address),

    EventWinnerCount(u64),
    EventWinnerAt(u64, u32),

    ContributorAmount(u64, Address),
    ContributorCount(u64),
    ContributorAt(u64, u32),
    ContributorSlot(u64, Address),

    MilestoneClaimed(u64, Address, u32),

    CrowdfundingMilestonesClaimed(u64),

    CancellationState(u64),

    Version,
    PendingUpgrade,
    MigratedToVersion,

    OpSeen(BytesN<32>),

    SupportedTokenCount,
    SupportedTokenAt(u32),
    SupportedTokenSlot(Address),

    // Appended in 1.2.0 to preserve existing key discriminants.
    NonOwnerContributionTotal(u64),

    // Appended in 1.3.0 to preserve existing key discriminants.
    EventPrizeAward(u64, u32),
    EventUnclaimedPrizes(u64),
    EventPrizeBaseEscrow(u64),
    EventPrizeClaimExpiry(u64),
}

// ============================================================
// PRIZE AWARD payload (keyed by (event, position); pull-model claims)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrizeAward {
    pub recipient: Address,
    pub anchor_idx: u32,
    pub reputation_bump: u32,
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

// ============================================================
// PENDING UPGRADE (timelocked wasm rotation, H6)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub new_version: String,
    pub proposed_at_ledger: u32,
    pub available_at_ledger: u32,
    pub expires_at_ledger: u32,
}
