use soroban_sdk::contracterror;

// Retired codes are never reused, so a code in an old log or client keeps its
// meaning: 2-4, 10, 11, 21, 36-38, 40-43, 59, 80.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    InvalidFeeBps = 5,
    NotInitialized = 6,

    // Shared by both two-step rotations (admin and event manager): no pending
    // proposal / target mismatch (12) and pending proposal expired (13). The
    // manager flow reuses these because the two flows are structurally
    // identical, not to duplicate a variant per flow.
    PendingRotationMismatch = 12,
    PendingRotationExpired = 13,

    TokenNotSupported = 20,

    EventNotFound = 30,
    EventNotActive = 31,
    InvalidPillar = 32,
    InvalidReleaseKind = 33,
    InvalidDistribution = 34,
    InvalidBudget = 35,
    TitleTooLong = 39,

    NoSubmissions = 50,
    InvalidWinnerPosition = 51,
    DuplicateWinnerPosition = 52,
    DistributionMismatch = 53,
    MilestoneAlreadyClaimed = 54,
    InvalidMilestone = 55,
    InsufficientEscrow = 56,
    WinnersAlreadySelected = 90,

    BelowMinimumContribution = 57,
    InvalidContributionAmount = 58,

    OpAlreadySeen = 60,

    // A u32 overflow on the contributor index; contributor sets have no cap.
    TooManyContributors = 61,

    CancellationNotStarted = 62,
    CancellationAlreadyStarted = 63,
    CancellationNotFinished = 64,
    CancellationTotalMissing = 66,

    UpgradeNotProposed = 65,
    UpgradeTimelockNotElapsed = 67,
    UpgradeProposalExpired = 68,
    MigrationAlreadyApplied = 69,

    Paused = 70,
    EventIdOverflow = 71,

    // contracterror caps at 50 cases. Discriminants are not dense: 91 is a
    // numeric label, not the case count.
    PrizeAlreadyClaimed = 91,
    DuplicateRecipient = 92,
    ReputationBumpTooLarge = 93,
    InvalidBatchSize = 94,
    NoRefundOwed = 95,
    AwardsOutstanding = 96,
    MigrationIncomplete = 97,
}
