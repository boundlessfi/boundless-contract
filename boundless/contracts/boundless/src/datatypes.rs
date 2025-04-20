use soroban_sdk::{contracttype, contracterror, Address, String, Vec};

// Constants
pub(crate) const DAY_IN_LEDGERS: u32 = 17280; // Assuming 5 seconds per ledger
pub(crate) const PROJECTS_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const PROJECTS_LIFETIME_THRESHOLD: u32 = PROJECTS_BUMP_AMOUNT - DAY_IN_LEDGERS;

// Fixed periods in days
pub(crate) const FUNDING_PERIOD_DAYS: u32 = 30;
pub(crate) const VOTING_PERIOD_DAYS: u32 = 30;

// Convert days to ledgers
pub(crate) const FUNDING_PERIOD_LEDGERS: u32 = FUNDING_PERIOD_DAYS * DAY_IN_LEDGERS;
pub(crate) const VOTING_PERIOD_LEDGERS: u32 = VOTING_PERIOD_DAYS * DAY_IN_LEDGERS;

/// Enum representing all possible data keys for contract storage
#[contracttype]
pub enum DataKey {
    Version,
    Initialized,
    Admin,
    Projects,
    Project(String),
    Backers(String),
    Votes(String),
    Milestones(String),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BoundlessError {
    /// Contract has already been initialized
    AlreadyInitialized = 1,
    /// Operation is not authorized
    Unauthorized = 2,
    /// Project already exists
    AlreadyExists = 3,
    /// Project does not exist
    NotFound = 4,
    /// Invalid funding target (must be greater than 0)
    InvalidFundingTarget = 5,
    /// Invalid milestone number
    InvalidMilestone = 6,
    /// Project is closed
    ProjectClosed = 7,
    /// Funding period has ended
    FundingPeriodEnded = 8,
    /// Voting period has ended
    VotingPeriodEnded = 9,
    /// User has already voted
    AlreadyVoted = 10,
    /// User has not voted
    NotVoted = 11,
    /// Invalid vote value
    InvalidVote = 12,
    /// Milestone has already been released
    MilestoneAlreadyReleased = 13,
    /// Milestone has already been approved
    MilestoneAlreadyApproved = 14,
    /// Milestone has already been rejected
    MilestoneAlreadyRejected = 15,
    /// Insufficient funds
    InsufficientFunds = 16,
    /// Refund already processed
    RefundAlreadyProcessed = 17,
    /// Invalid operation for current project status
    InvalidOperation = 18,
    /// Internal error
    InternalError = 19,
}

/// Enum representing the current status of a project
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum ProjectStatus {
    /// Project is in funding phase
    Funding = 1,
    /// Project is in voting phase
    Voting = 2,
    /// Project has been successfully funded
    Funded = 3,
    /// Project funding failed
    Failed = 4,
    /// Project has been closed by creator
    Closed = 5,
}

/// Enum representing the status of a milestone
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum MilestoneStatus {
    /// Milestone has not been released yet
    Pending,
    /// Milestone has been released for approval
    Released,
    /// Milestone has been approved
    Approved,
    /// Milestone has been rejected
    Rejected,
}

// Event Structures
#[contracttype]
#[derive(Clone)]
pub struct ProjectCreatedEvent {
    pub project_id: String,
    pub creator: Address,
    pub funding_target: u64,
    pub funding_deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProjectFundedEvent {
    pub project_id: String,
    pub total_funded: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProjectVotingEvent {
    pub project_id: String,
    pub voting_deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProjectClosedEvent {
    pub project_id: String,
    pub is_successful: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct MilestoneReleasedEvent {
    pub project_id: String,
    pub milestone_number: u32,
    pub amount: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct MilestoneApprovedEvent {
    pub project_id: String,
    pub milestone_number: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct MilestoneRejectedEvent {
    pub project_id: String,
    pub milestone_number: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct RefundProcessedEvent {
    pub project_id: String,
    pub backer: Address,
    pub amount: u64,
}

/// Struct representing a milestone in a project
#[derive(Clone)]
#[contracttype]
pub struct Milestone {
    /// Milestone number (1-based index)
    pub number: u32,
    /// Description of the milestone
    pub description: String,
    /// Amount to be released for this milestone
    pub amount: u64,
    /// Current status of the milestone
    pub status: MilestoneStatus,
    /// Timestamp when the milestone was released
    pub released_at: Option<u64>,
    /// Timestamp when the milestone was approved/rejected
    pub completed_at: Option<u64>,
}

/// Struct representing a backer's contribution to a project
#[derive(Clone)]
#[contracttype]
pub struct BackerContribution {
    /// Backer's address
    pub backer: Address,
    /// Amount contributed
    pub amount: u64,
    /// Timestamp of contribution
    pub timestamp: u64,
}

/// Struct representing a vote on a project
#[derive(Clone)]
#[contracttype]
pub struct Vote {
    /// Voter's address
    pub voter: Address,
    /// Vote value (positive for approval, negative for rejection)
    pub value: i32,
    /// Timestamp of vote
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct Project {
    pub project_id: String,
    pub creator: Address,
    pub metadata_uri: String,
    pub funding_target: u64,
    pub milestone_count: u32,
    pub current_milestone: u32,
    pub total_funded: u64,
    pub backers: Vec<(Address, u64)>,
    pub votes: Vec<(Address, i32)>,
    pub validated: bool,
    pub is_successful: bool,
    pub is_closed: bool,
    pub created_at: u64,
    pub milestone_approvals: Vec<(u32, bool)>,
    pub milestone_releases: Vec<(u32, u64)>,
    pub refund_processed: bool,
    pub funding_deadline: u64,
    pub voting_deadline: u64,
    /// Current status of the project
    pub status: ProjectStatus,
    /// Milestones for this project
    pub milestones: Vec<Milestone>,
}

#[contracttype]
pub enum ContractDataKey {
    Version,
    Initialized,
    Admin,
}

#[contracttype]
pub enum ProjectDataKey {
    Projects,
    Project(String),
} 
