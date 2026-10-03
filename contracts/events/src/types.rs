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

    NonOwnerContributionTotal(u64),

    EventPrizeAward(u64, u32),
    EventUnclaimedPrizes(u64),
    EventPrizeClaimExpiry(u64),

    PendingManager(u64),

    // Sum of awarded-but-unclaimed prizes. `remaining_escrow` only drops at
    // claim time, so without this a second selection would see funds an
    // earlier winner is still entitled to and could promise them twice.
    EventOwedTotal(u64),

    // Next event id `migrate_events` has yet to convert. The pass is paged
    // because one invocation may touch only 100 ledger entries, so a
    // deployment with real history cannot be migrated in a single call.
    MigrationCursor,

    // Grant awards by recipient, written once at selection, and each
    // recipient's release progress. A release reads these instead of walking
    // the winner rows, which grow with every payment. Grants selected before
    // these keys existed have no roster and keep the row walk.
    GrantRoster(u64),
    GrantProgress(u64, Address),

    // A cancel refund the contributor's account could not take (frozen, or no
    // trustline). Held for `claim_refund` so one account cannot stall the
    // crank for everyone behind it.
    UnclaimedRefund(u64, Address),

    // When the current pause began, and the seconds spent paused before it.
    // Claim windows run on time the contract was open, so a pause cannot run
    // a winner's window out while they are unable to claim.
    PausedAt,
    PausedSeconds,

    // Co-signs crowdfunding milestone releases in place of the admin, so the
    // keys that can upgrade the contract are not needed for routine payouts.
    ReleaseValidator,
    PendingReleaseValidator,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrantAward {
    pub position: u32,
    pub amount: i128,
}

/// Milestones a recipient has settled, released or forfeited, and what they
/// were worth. The last settlement takes whatever is left of the award.
#[contracttype]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GrantProgress {
    pub settled: u32,
    pub settled_amount: i128,
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingValidator {
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
// PENDING UPGRADE (timelocked wasm rotation)
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
