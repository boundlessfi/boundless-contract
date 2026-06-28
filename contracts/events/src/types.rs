// boundless-events: on-chain types.
//
// Spec: boundless-platform-contract-prd.md Sections 5.1 to 5.2.

use soroban_sdk::{contracttype, Address, BytesN, Map, String};

// ============================================================
// PILLAR
//
// Crowdfunding differs from the other three pillars:
//   - The owner is the project builder, not an organization.
//   - There is no upfront owner deposit; the escrow starts at 0 and grows
//     via add_funds from community backers.
//   - There is exactly one recipient (the builder, registered as Winner at
//     position 1 at create time).
//   - claim_milestone uses dynamic math: amount = remaining_escrow /
//     remaining_milestones, so each release pays a fair share of whatever
//     the campaign actually raised.
// Spec: boundless-crowdfunding-prd.md.
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
//
// Cancelling is the intermediate state once start_cancel has snapshotted
// the contributor list. The event is frozen (no add_funds, submit,
// claim_milestone, select_winners) until finalize_cancel flips it to
// Cancelled. New variant added at the tail so prior on-chain variant
// indices stay stable.
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
//
// Paged cancel: see docs/audit-2026-06-stellar-skill.md H3+H4
// follow-up. start_cancel snapshots the refund math once so that
// concurrent contributor mutations (none possible while Cancelling, but
// belt-and-suspenders) can't bias the per-batch payouts.
//
// Branch:
//   OwnerOnly       - no partner contributions; owner gets all of remaining_at_start.
//   FullPartnerThenResidual - remaining >= non_owner_total; each partner gets full
//                             amount, owner gets remaining - non_owner_total.
//   ProRataPartners - remaining < non_owner_total; partners get
//                     floor(amt * remaining / non_owner_total); owner gets 0.
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
    // Optional management authority. None => owner manages (legacy behavior).
    // When set, this address authorizes select_winners + cancel instead of the
    // owner, so funding source and management identity can differ.
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
// Input to select_winners. The orchestrator computes reputation_bump off-chain
// per the policy tables in boundless-credits-reputation-prd.md Section 9; the
// contract just records.
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

    // Per-event management authority override. When present, this address
    // (not event.owner) authorizes select_winners + cancel. Lets an org fund
    // from any wallet (owner) while keeping management bound to its canonical
    // wallet. Absent => management falls back to event.owner (legacy events).
    EventManager(u64),

    // Per-event applicant list. Paged: ApplicantCount + ApplicantAt(idx).
    // Slot index (1-based) for O(1) membership / O(1) swap-remove. Slot of
    // 0 means absent. Caps at MAX_APPLICANTS_PER_EVENT to keep cancel-time
    // refund passes inside Soroban's per-tx footprint budget.
    EventApplicantCount(u64),
    EventApplicantAt(u64, u32),
    EventApplicantSlot(u64, Address),

    // Per-event submission, keyed by (event_id, applicant). No iteration
    // surface — submissions are looked up by address only.
    EventSubmission(u64, Address),

    // Per-event winner list. WinnerCount + WinnerAt(idx). No slot index
    // because select_winners enforces uniqueness on position, and the
    // claim_milestone path needs to enumerate anyway.
    EventWinnerCount(u64),
    EventWinnerAt(u64, u32),

    // Partner contributions to an event's escrow.
    //   ContributorAmount(id, addr)     -> i128 running total
    //   ContributorCount(id)            -> u32 number of distinct contributors
    //   ContributorAt(id, idx)          -> Address at slot idx
    //   ContributorSlot(id, addr)       -> u32 1-based slot index; 0 absent
    // Owner deposits do NOT appear here; they're tracked via event.owner /
    // event.total_budget.
    ContributorAmount(u64, Address),
    ContributorCount(u64),
    ContributorAt(u64, u32),
    ContributorSlot(u64, Address),

    // Grant milestone tracking: (event_id, recipient, milestone) -> bool
    MilestoneClaimed(u64, Address, u32),

    // Crowdfunding: count of milestones already claimed against an event.
    // Used by the dynamic-payout math (amount = remaining_escrow /
    // (total_milestones - claimed_count)). Only written/read for
    // Pillar::Crowdfunding; absent entries default to 0.
    CrowdfundingMilestonesClaimed(u64),

    // Paged cancellation cursor; present iff event.status == Cancelling.
    CancellationState(u64),

    // H6: contract semver, timelocked upgrade slot, last migrated-to version.
    //
    //   Version            -> String. Set by __constructor; bumped by
    //                          apply_upgrade(). Exposed via version().
    //   PendingUpgrade     -> PendingUpgrade struct. Present between
    //                          propose_upgrade and apply_upgrade /
    //                          cancel_pending_upgrade.
    //   MigratedToVersion  -> String. Records the last Version that
    //                          successfully completed migrate(); guards
    //                          against double-running the same migration.
    Version,
    PendingUpgrade,
    MigratedToVersion,

    // Idempotency
    OpSeen(BytesN<32>),

    // Enumerable whitelist index (parallels the SupportedToken bool above).
    // Lets the full whitelist be read authoritatively from state via
    // supported_token_count + supported_token_at, instead of replaying the
    // ephemeral TokenRegistered / TokenDeregistered events (which the RPC only
    // retains for a short window). Appended here to keep existing keys stable.
    //   SupportedTokenCount     -> u32 number of whitelisted tokens
    //   SupportedTokenAt(idx)   -> Address at slot idx, idx in [0, count)
    //   SupportedTokenSlot(addr)-> u32 1-based slot; 0 means absent
    SupportedTokenCount,
    SupportedTokenAt(u32),
    SupportedTokenSlot(Address),
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
//
// propose_upgrade writes a row carrying the proposed wasm hash and the
// new_version label the contract will report after apply_upgrade. The
// proposal is timelocked: apply_upgrade can only fire after
// available_at_ledger and before expires_at_ledger.
//
// new_version is stored upfront so the apply step is purely mechanical;
// off-chain monitoring can see exactly which version the proposal upgrades
// to without inspecting the new wasm.
//
// Spec: docs/audit-2026-06-stellar-skill.md H6.
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
