use soroban_sdk::{Address, BytesN, Env, String, Vec};

use crate::datatypes::{BoundlessError, Milestone, MilestoneStatus, Project, ProjectStatus};

pub trait ContractManagement {
    fn initialize(env: Env, admin: Address) -> Result<(), BoundlessError>;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), BoundlessError>;
    fn get_admin(e: &Env) -> Address;
    fn get_version(e: &Env) -> u32;
}

pub trait ProjectManagement {
    fn create_project(
        env: Env,
        project_id: String,
        creator: Address,
        metadata_uri: String,
        funding_target: u64,
        milestone_count: u32,
    ) -> Result<(), BoundlessError>;
    fn get_project(env: Env, project_id: String) -> Result<Project, BoundlessError>;
    fn update_project_metadata(
        env: Env,
        project_id: String,
        creator: Address,
        new_metadata_uri: String,
    ) -> Result<(), BoundlessError>;
    fn update_project_milestone_count(
        env: Env,
        project_id: String,
        creator: Address,
        new_milestone_count: u32,
    ) -> Result<(), BoundlessError>;
    fn modify_milestone(
        env: Env,
        project_id: String,
        caller: Address,
        new_milestone_count: u32,
    ) -> Result<(), BoundlessError>;
    fn close_project(env: Env, project_id: String, creator: Address) -> Result<(), BoundlessError>;
    fn get_project_status(env: Env, project_id: String) -> Result<ProjectStatus, BoundlessError>;
    fn list_projects(env: Env) -> Result<Vec<String>, BoundlessError>;
    fn get_project_stats(env: Env, project_id: String) -> Result<(u64, u64, u32), BoundlessError>;
}

pub trait VotingOperations {
    fn vote_project(
        env: Env,
        project_id: String,
        voter: Address,
        vote_value: i32,
    ) -> Result<(), BoundlessError>;
    fn withdraw_vote(env: Env, project_id: String, voter: Address) -> Result<(), BoundlessError>;
    fn has_voted(env: Env, project_id: String, voter: Address) -> Result<bool, BoundlessError>;
    fn get_vote(env: Env, project_id: String, voter: Address) -> Result<i32, BoundlessError>;
}

pub trait MilestoneOperations {
    fn release_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn approve_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn reject_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError>;
    fn get_milestone_status(
        env: Env,
        project_id: String,
        milestone_number: u32,
    ) -> Result<MilestoneStatus, BoundlessError>;
    fn get_project_milestones(
        env: Env,
        project_id: String,
    ) -> Result<Vec<Milestone>, BoundlessError>;
}

pub trait FundingOperations {
    fn fund_project(
        env: Env,
        project_id: String,
        amount: i128,
        funder: Address,
        token_contract: Address,
    ) -> Result<(), BoundlessError>;
    fn refund(env: Env, project_id: String, token_contract: Address) -> Result<(), BoundlessError>;
    fn get_project_funding(env: Env, project_id: String) -> Result<(u64, u64), BoundlessError>;
    fn get_backer_contribution(
        env: Env,
        project_id: String,
        backer: Address,
    ) -> Result<u64, BoundlessError>;
    fn whitelist_token_contract(
        env: Env,
        admin: Address,
        project_id: String,
        token_contract: Address,
    ) -> Result<(), BoundlessError>;
}
