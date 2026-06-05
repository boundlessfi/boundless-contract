// boundless-profile: error codes.
//
// Spec: boundless-credits-reputation-prd.md Section 8.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    AdminCannotBeZero = 2,
    EventsContractNotConfigured = 3,
    Unauthorized = 4,
    NotAdmin = 5,
    PendingAdminMismatch = 6,
    PendingAdminExpired = 7,
    NotInitialized = 8,
    EventsContractAlreadyConfigured = 14,
    PendingEventsContractMismatch = 15,
    PendingEventsContractExpired = 16,
    PendingEventsContractTimelock = 17,

    ProfileNotFound = 10,
    InsufficientCredits = 11,
    InvalidAmount = 12,
    ReasonRequired = 13,

    OpAlreadySeen = 20,
    Paused = 30,

    // H6: timelocked upgrade + migration
    UpgradeNotProposed = 40,
    UpgradeTimelockNotElapsed = 41,
    UpgradeProposalExpired = 42,
    MigrationAlreadyApplied = 43,
}
