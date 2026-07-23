use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    AdminCannotBeZero = 2,
    FeeAccountCannotBeZero = 3,
    ProfileContractCannotBeZero = 4,
    InvalidFeeBps = 5,
    NotInitialized = 6,

    Unauthorized = 10,
    NotAdmin = 11,
    // Shared by both two-step rotations (admin and event manager): no pending
    // proposal / target mismatch (12) and pending proposal expired (13). The
    // enum is at the 50-case XDR cap, so the manager flow reuses these rather
    // than adding variants.
    PendingRotationMismatch = 12,
    PendingRotationExpired = 13,

    TokenNotSupported = 20,
    FeeAccountMissingTrustline = 21,

    EventNotFound = 30,
    EventNotActive = 31,
    InvalidPillar = 32,
    InvalidReleaseKind = 33,
    InvalidDistribution = 34,
    InvalidBudget = 35,
    // 36-38 retired: deadline enforcement removed (submission windows are an
    // off-chain/backend concern; the contract no longer gates on deadline).
    TitleTooLong = 39,

    ApplicantAlreadyApplied = 40,
    ApplicantNotApplied = 41,
    SubmissionNotFound = 42,
    SubmissionAlreadyExists = 43,

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

    TooManyApplicants = 59,

    OpAlreadySeen = 60,

    // Also returned by append_submission's cap check — the enum is at
    // the 50-case XDR cap, so the hackathon submission cap reuses this
    // rather than adding a variant.
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

    ProfileCallFailed = 80,

    // Enum is at the 50-case XDR cap; consolidate before adding another.
    PrizeAlreadyClaimed = 91,
}
