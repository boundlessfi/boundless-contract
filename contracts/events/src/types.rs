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
    /// Advertised minimum per position, in token-native units. `select_winners`
    /// may pay above a floor but never below it, so a published prize table is
    /// a guarantee rather than an estimate. Positions absent from the map carry
    /// no floor and are payable at any positive amount.
    pub prize_floors: Map<u32, i128>,
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
    pub prize_floors: Map<u32, i128>,
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
    /// Award amount in token-native units. Allocation happens here, at payout,
    /// not at create: the event holds a pool and each selection names what it
    /// spends from it.
    pub amount: i128,
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

    /// Pre-1.7.0 single-submission key. Read only by `migrate`, which moves
    /// each row to slot 0 of `EventSubmissionEntry`. Never write this.
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

    // Temporary idempotency flag keyed by (authorizing caller, op_id) so a
    // permissionless entrypoint cannot squat a privileged one's op_id.
    OpSeen(Address, BytesN<32>),

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

    // Appended for two-step manager rotation to preserve key discriminants.
    PendingManager(u64),

    // Appended to cap per-event submission storage growth (security fix).
    EventSubmissionCount(u64),

    // Appended in 1.7.0. Sum of awarded-but-unclaimed prizes. `remaining_escrow`
    // only drops at claim time, so without this a second selection would see
    // funds an earlier winner is still entitled to and could promise them twice.
    EventOwedTotal(u64),

    // Appended in 1.7.0. Submissions gain a caller-chosen slot, so one wallet
    // may hold several distinct entries in one event. The contract assigns no
    // meaning to the slot; callers use it for whatever separates their entries.
    EventSubmissionEntry(u64, Address, u32),
    EventApplicantSubmissionCount(u64, Address),

    // Next event id `migrate_events` has yet to convert. The pass is paged
    // because one invocation may touch only 100 ledger entries, so a
    // deployment with real history cannot be migrated in a single call.
    MigrationCursor,
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
// PENDING MANAGER payload (target + expiry ledger)
// ============================================================
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingManager {
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
