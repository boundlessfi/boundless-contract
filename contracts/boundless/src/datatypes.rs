use soroban_sdk::{contracterror, contractevent, contracttype, Address, Symbol, Vec};

#[contracttype]
pub enum DataKey {
    Version,
    Initialized,
    Admin,
    // New entity types
    Campaigns,
    Campaign(u64),
    Grants,
    Grant(u64),
    Hackathons,
    Hackathon(u64),
    // Entity-specific data
    CampaignBackers(u64),
    GrantApplications(u64),
    HackathonEntries(u64),
    HackathonJudges(u64),
    // Escrow linking
    CampaignEscrow(u64),       // Links campaign to escrow contract
    EscrowValidation(Address), // Validates escrow contracts
}

// Campaign/Grant/Hackathon Status
#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum Status {
    Pending,
    Active,
    Completed,
    Failed,
}

// Milestone Status
#[derive(Clone, PartialEq, Debug)]
#[contracttype]
pub enum MilestoneStatus {
    Pending,
    Approved,
    Rejected,
    Released,
}

// Entity types that can have milestones
#[derive(Clone)]
#[contracttype]
pub enum EntityType {
    Campaign,
    Grant,
    Hackathon,
}

// Campaign struct
#[derive(Clone)]
#[contracttype]
pub struct Campaign {
    pub id: u64,
    pub owner: Address,
    pub title: Symbol,
    pub description: Symbol,
    pub funding_goal: i128,
    pub escrow_contract_id: Address, // Trustless Work escrow contract ID
    pub milestones: Vec<Milestone>,
    pub backers: Vec<Backer>,
    pub status: Status,
}

// Grant struct
#[derive(Clone)]
#[contracttype]
pub struct Grant {
    pub id: u64,
    pub sponsor: Address,
    pub title: Symbol,
    pub description: Symbol,
    pub prize_pool: i128,
    pub winners: u32,
    pub milestones: Vec<Milestone>,
    pub applications: Vec<GrantApplication>,
    pub selected_winners: Vec<Address>,
    pub status: Status,
}

// Hackathon struct
#[derive(Clone)]
#[contracttype]
pub struct Hackathon {
    pub id: u64,
    pub organizer: Address,
    pub title: Symbol,
    pub description: Symbol,
    pub theme: Symbol,
    pub prize_pool: i128,
    pub milestones: Vec<Milestone>,
    pub entries: Vec<HackathonEntry>,
    pub judges: Vec<Judge>,
    pub selected_winners: Vec<Address>,
    pub status: Status,
}

// Milestone struct - now generic for all entity types
#[derive(Clone)]
#[contracttype]
pub struct Milestone {
    pub id: u64,
    pub entity_id: u64,          // ID of the campaign/grant/hackathon
    pub entity_type: EntityType, // Type of entity this milestone belongs to
    pub description: Symbol,
    pub amount: i128,
    pub status: MilestoneStatus,
}

// Backer struct
#[derive(Clone)]
#[contracttype]
pub struct Backer {
    pub wallet: Address,
    pub amount: i128,
}

// Grant application struct
#[derive(Clone)]
#[contracttype]
pub struct GrantApplication {
    pub project: Symbol,
    pub applicant: Address,
    pub submission_time: u64,
}

// Hackathon entry struct
#[derive(Clone)]
#[contracttype]
pub struct HackathonEntry {
    pub project: Symbol,
    pub participant: Address,
    pub submission_time: u64,
    pub score: Option<u32>,
}

// Judge struct for hackathons
#[derive(Clone)]
#[contracttype]
pub struct Judge {
    pub address: Address,
    pub name: Symbol,
}

// Errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BoundlessError {
    /// Invalid user authorization for action
    Unauthorized = 1,
    /// Campaign with the given ID does not exist
    CampaignNotFound = 2,
    /// Grant with the given ID does not exist
    GrantNotFound = 3,
    /// Hackathon with the given ID does not exist
    HackathonNotFound = 4,
    /// Milestone with the given ID does not exist
    MilestoneNotFound = 5,
    /// Invalid status for the operation
    InvalidStatus = 6,
    /// Invalid entity type
    InvalidEntityType = 7,
    /// Escrow operation failed
    EscrowError = 8,
    /// Milestone operation failed
    MilestoneError = 9,
    /// Resource not found
    NotFound = 10,
    /// Invalid operation for current status
    InvalidOperation = 11,
    /// Already initialized
    AlreadyInitialized = 12,
    /// Escrow contract validation failed
    InvalidEscrowContract = 13,
    /// Escrow contract already linked
    EscrowAlreadyLinked = 14,
    /// Escrow contract not found
    EscrowNotFound = 15,
}
// Events
#[contractevent]
pub struct CampaignCancelled {
    pub campaign_id: u64,
    pub admin: Address,
}

#[contractevent]
pub struct CampaignStatusUpdated {
    pub campaign_id: u64,
    pub status: Status,
    pub admin: Address,
}

#[contractevent]
pub struct FundsReleased {
    pub campaign_id: u64,
    pub milestone_id: u64,
    pub amount: i128,
    pub releaser: Address,
}
