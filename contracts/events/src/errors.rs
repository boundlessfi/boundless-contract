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
    // manager flow reuses these because the two flows are structurally
    // identical, not to duplicate a variant per flow.
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

    // 40-43 retired in 2.0.0 with the on-chain participation records
    // (ApplicantAlreadyApplied, ApplicantNotApplied, SubmissionNotFound,
    // SubmissionAlreadyExists). Left as a gap rather than reused so an old
    // client decoding a stored error code never reads the wrong case.
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

    // 59 retired in 2.0.0 with the applicant index (TooManyApplicants).
    OpAlreadySeen = 60,

    // Signals a u32 counter overflow on the contributor index. Per-event
    // participant caps were removed (contributor sets are unbounded; entries
    // are per-contributor and self-funded).
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

    ProfileCallFailed = 80,

    // contracterror caps at 50 cases (48 used). Discriminants are not dense —
    // 91 is a numeric label, not the case count.
    PrizeAlreadyClaimed = 91,
}
