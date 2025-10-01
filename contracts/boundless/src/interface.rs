use soroban_sdk::{Address, BytesN, Env, Symbol, Vec};

use crate::datatypes::{BoundlessError, EntityType, Milestone, MilestoneStatus};

pub trait ContractManagement {
    fn initialize(env: Env, admin: Address) -> Result<(), BoundlessError>;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), BoundlessError>;
    fn get_admin(e: &Env) -> Address;
    fn get_version(e: &Env) -> u32;
}

pub trait CampaignManagement {
    fn create_campaign(
        env: Env,
        owner: Address,
        title: Symbol,
        description: Symbol,
        goal: i128,
        escrow_contract_id: Address, // Trustless Work escrow contract ID
        milestones: Vec<crate::datatypes::Milestone>, // Milestone data
    ) -> Result<u64, BoundlessError>;
    fn fund_campaign(
        env: Env,
        campaign_id: u64,
        backer: Address,
        amount: i128,
    ) -> Result<(), BoundlessError>;
    fn release_funds(env: Env, campaign_id: u64, milestone_id: u64) -> Result<(), BoundlessError>;
    fn get_campaign(
        env: Env,
        campaign_id: u64,
    ) -> Result<crate::datatypes::Campaign, BoundlessError>;

    // Lifecycle management
    fn complete_campaign(env: Env, campaign_id: u64, admin: Address) -> Result<(), BoundlessError>;
    fn cancel_campaign(env: Env, campaign_id: u64, admin: Address) -> Result<(), BoundlessError>;

    // Status and participant management
    fn update_campaign_status(
        env: Env,
        campaign_id: u64,
        status: crate::datatypes::Status,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn get_campaign_backers(
        env: Env,
        campaign_id: u64,
    ) -> Result<Vec<crate::datatypes::Backer>, BoundlessError>;
}

pub trait GrantManagement {
    fn create_grant(
        env: Env,
        sponsor: Address,
        title: Symbol,
        description: Symbol,
        pool: i128,
        winners: u32,
    ) -> Result<u64, BoundlessError>;
    fn apply_to_grant(env: Env, grant_id: u64, project: Symbol) -> Result<(), BoundlessError>;
    fn get_grant(env: Env, grant_id: u64) -> Result<crate::datatypes::Grant, BoundlessError>;

    // Lifecycle management
    fn complete_grant(env: Env, grant_id: u64, admin: Address) -> Result<(), BoundlessError>;
    fn cancel_grant(env: Env, grant_id: u64, admin: Address) -> Result<(), BoundlessError>;

    // Winner selection and management
    fn select_grant_winners(
        env: Env,
        grant_id: u64,
        winners: Vec<Address>,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn get_grant_applications(env: Env, grant_id: u64) -> Result<Vec<Symbol>, BoundlessError>;
    fn get_grant_winners(env: Env, grant_id: u64) -> Result<Vec<Address>, BoundlessError>;

    // Status management
    fn update_grant_status(
        env: Env,
        grant_id: u64,
        status: crate::datatypes::Status,
        admin: Address,
    ) -> Result<(), BoundlessError>;
}

pub trait HackathonManagement {
    // Creation and basic operations
    fn create_hackathon(
        env: Env,
        organizer: Address,
        title: Symbol,
        description: Symbol,
        theme: Symbol,
        prize_pool: i128,
    ) -> Result<u64, BoundlessError>;
    fn submit_hackathon_entry(
        env: Env,
        hackathon_id: u64,
        project: Symbol,
    ) -> Result<(), BoundlessError>;
    fn get_hackathon(
        env: Env,
        hackathon_id: u64,
    ) -> Result<crate::datatypes::Hackathon, BoundlessError>;

    // Lifecycle management
    fn complete_hackathon(
        env: Env,
        hackathon_id: u64,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn cancel_hackathon(env: Env, hackathon_id: u64, admin: Address) -> Result<(), BoundlessError>;

    // Judging and winner selection
    fn judge_hackathon_entry(
        env: Env,
        hackathon_id: u64,
        project: Symbol,
        score: u32,
        judge: Address,
    ) -> Result<(), BoundlessError>;
    fn select_hackathon_winners(
        env: Env,
        hackathon_id: u64,
        winners: Vec<Address>,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn get_hackathon_entries(env: Env, hackathon_id: u64) -> Result<Vec<Symbol>, BoundlessError>;
    fn get_hackathon_winners(env: Env, hackathon_id: u64) -> Result<Vec<Address>, BoundlessError>;

    // Status management
    fn update_hackathon_status(
        env: Env,
        hackathon_id: u64,
        status: crate::datatypes::Status,
        admin: Address,
    ) -> Result<(), BoundlessError>;

    // Judge management
    fn add_hackathon_judge(
        env: Env,
        hackathon_id: u64,
        judge: Address,
        name: Symbol,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn remove_hackathon_judge(
        env: Env,
        hackathon_id: u64,
        judge: Address,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn get_hackathon_judges(
        env: Env,
        hackathon_id: u64,
    ) -> Result<Vec<crate::datatypes::Judge>, BoundlessError>;
}

pub trait MilestoneManagement {
    // Generic milestone operations that work with any entity type
    fn release_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
    ) -> Result<(), BoundlessError>;
    fn update_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
        status: MilestoneStatus,
    ) -> Result<(), BoundlessError>;
    fn approve_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
        approver: Address,
    ) -> Result<(), BoundlessError>;
    fn reject_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
        rejector: Address,
    ) -> Result<(), BoundlessError>;
    fn raise_dispute(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
        reason: Symbol,
    ) -> Result<(), BoundlessError>;

    // Milestone creation for different entity types
    fn create_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        description: Symbol,
        amount: i128,
    ) -> Result<u64, BoundlessError>;

    // Milestone queries
    fn get_milestone(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
        milestone_id: u64,
    ) -> Result<Milestone, BoundlessError>;
    fn get_entity_milestones(
        env: Env,
        entity_id: u64,
        entity_type: EntityType,
    ) -> Result<Vec<Milestone>, BoundlessError>;
}

pub trait EscrowManagement {
    fn link_escrow(
        env: Env,
        campaign_id: u64,
        escrow_contract_id: Address,
    ) -> Result<(), BoundlessError>;
    fn get_escrow_contract(env: Env, campaign_id: u64) -> Result<Address, BoundlessError>;
    fn validate_escrow_contract(
        env: Env,
        escrow_contract_id: Address,
    ) -> Result<bool, BoundlessError>;
}
