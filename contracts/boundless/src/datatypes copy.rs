use soroban_sdk::{contracttype, Address, String, Vec};

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
}

// Constants for time periods
pub(crate) const DAY_IN_LEDGERS: u32 = 17280; // Assuming 5 seconds per ledger
pub(crate) const PROJECTS_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub(crate) const PROJECTS_LIFETIME_THRESHOLD: u32 = PROJECTS_BUMP_AMOUNT - DAY_IN_LEDGERS;

// Fixed periods in days
pub(crate) const FUNDING_PERIOD_DAYS: u32 = 30;
pub(crate) const VOTING_PERIOD_DAYS: u32 = 30;

// Convert days to ledgers
pub(crate) const FUNDING_PERIOD_LEDGERS: u32 = FUNDING_PERIOD_DAYS * DAY_IN_LEDGERS;
pub(crate) const VOTING_PERIOD_LEDGERS: u32 = VOTING_PERIOD_DAYS * DAY_IN_LEDGERS; 
