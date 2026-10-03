use soroban_sdk::contracterror;

// Retired codes are never reused, so a code in an old log or client keeps its
// meaning: 4, 5.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    AdminCannotBeZero = 2,
    EventsContractNotConfigured = 3,
    PendingAdminMismatch = 6,
    PendingAdminExpired = 7,
    NotInitialized = 8,
    EventsContractAlreadyConfigured = 14,
    PendingEventsContractMismatch = 15,
    PendingEventsContractExpired = 16,
    PendingEventsContractTimelock = 17,

    ProfileNotFound = 10,
    DeltaTooLarge = 11,
    InvalidAmount = 12,
    ReasonRequired = 13,

    OpAlreadySeen = 20,
    EarningsOverflow = 21,
    Paused = 30,

    UpgradeNotProposed = 40,
    UpgradeTimelockNotElapsed = 41,
    UpgradeProposalExpired = 42,
    MigrationAlreadyApplied = 43,
}
