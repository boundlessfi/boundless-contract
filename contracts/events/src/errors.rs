// boundless-events: error codes.
//
// Spec: boundless-platform-contract-prd.md Section 14.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // Init
    AlreadyInitialized = 1,
    AdminCannotBeZero = 2,
    FeeAccountCannotBeZero = 3,
    ProfileContractCannotBeZero = 4,
    InvalidFeeBps = 5,
    NotInitialized = 6,

    // Auth
    Unauthorized = 10,
    NotAdmin = 11,
    PendingAdminMismatch = 12,
    PendingAdminExpired = 13,

    // Token
    TokenNotSupported = 20,
    FeeAccountMissingTrustline = 21,

    // Event lifecycle
    EventNotFound = 30,
    EventNotActive = 31,
    InvalidPillar = 32,
    InvalidReleaseKind = 33,
    InvalidDistribution = 34,
    InvalidBudget = 35,
    DeadlineRequired = 36,
    DeadlinePassed = 37,
    DeadlineMustBeFuture = 38,
    TitleTooLong = 39,

    // Participation
    ApplicantAlreadyApplied = 40,
    ApplicantNotApplied = 41,
    SubmissionNotFound = 42,
    SubmissionAlreadyExists = 43,
    InsufficientCredits = 44,

    // Winners
    NoSubmissions = 50,
    InvalidWinnerPosition = 51,
    DuplicateWinnerPosition = 52,
    DistributionMismatch = 53,
    MilestoneAlreadyClaimed = 54,
    InvalidMilestone = 55,
    InsufficientEscrow = 56,
    WinnersAlreadySelected = 90,

    // Contributions
    BelowMinimumContribution = 57,
    InvalidContributionAmount = 58,

    // Capacity (per-event list caps; see MAX_*_PER_EVENT in event_ops)
    TooManyApplicants = 59,

    // Idempotency
    OpAlreadySeen = 60,

    // Capacity continued
    TooManyContributors = 61,

    // Paged cancellation flow
    CancellationNotStarted = 62,
    CancellationAlreadyStarted = 63,
    CancellationNotFinished = 64,

    UpgradeNotProposed = 65,
    UpgradeTimelockNotElapsed = 67,
    UpgradeProposalExpired = 68,
    MigrationAlreadyApplied = 69,

    // Pause
    Paused = 70,

    // Cross-contract
    ProfileCallFailed = 80,
}
