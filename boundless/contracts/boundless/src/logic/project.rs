use crate::{
    datatypes::{BoundlessError, DataKey, Project, ProjectStatus, VOTING_PERIOD_LEDGERS}, interface::ProjectManagement, BoundlessContract, BoundlessContractArgs, BoundlessContractClient
};
use soroban_sdk::{contractimpl, symbol_short, Address, Env, String, Vec};

#[contractimpl]
impl ProjectManagement for BoundlessContract {
    fn create_project(
        env: Env,
        project_id: String,
        creator: Address,
        metadata_uri: String,
        funding_target: u64,
        milestone_count: u32,
    ) -> Result<(), BoundlessError> {
        if env.storage().persistent().has(&DataKey::Project(project_id.clone())) {
            return Err(BoundlessError::AlreadyExists);
        }
        if funding_target == 0 {
            return Err(BoundlessError::InvalidFundingTarget);
        }
        if milestone_count <= 4 || milestone_count > 100 {
            return Err(BoundlessError::InvalidMilestone);
        }
        creator.require_auth();

        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + VOTING_PERIOD_LEDGERS as u64; // 14 days

        let project = Project {
            project_id: project_id.clone(),
            creator,
            metadata_uri,
            funding_target,
            milestone_count,
            current_milestone: 0,
            total_funded: 0,
            backers: Vec::new(&env),
            votes: Vec::new(&env),
            validated: false,
            is_successful: false,
            is_closed: false,
            created_at: current_time,
            milestone_approvals: Vec::new(&env),
            milestone_releases: Vec::new(&env),
            refund_processed: false,
            voting_deadline,
            funding_deadline: 0,
            milestones: Vec::new(&env),
            status: ProjectStatus::Voting,
        };

        env.storage().persistent().set(&DataKey::Project(project_id.clone()), &project);
        env.events().publish((DataKey::Project(project_id.clone()), symbol_short!("created")), project.project_id);
        Ok(())
    }
    fn get_project(env: Env, project_id: String) -> Result<Project, BoundlessError> {
        env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)
    }
    fn update_project_metadata(env: Env, project_id: String, creator: Address, new_metadata_uri: String) -> Result<(), BoundlessError> {
        creator.require_auth();
        let mut project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        if project.creator != creator {
            return Err(BoundlessError::Unauthorized);
        }
        project.metadata_uri = new_metadata_uri;
        env.storage().persistent().set(&DataKey::Project(project_id.clone()), &project);
        Ok(())
    }
    fn update_project_milestone_count(env: Env, project_id: String, creator: Address, new_milestone_count: u32) -> Result<(), BoundlessError> {
        creator.require_auth();
        let mut project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        if project.creator != creator {
            return Err(BoundlessError::Unauthorized);
        }
        project.milestone_count = new_milestone_count;
        env.storage().persistent().set(&DataKey::Project(project_id.clone()), &project);
        Ok(())
    }
    fn modify_milestone(env: Env, project_id: String, caller: Address, new_milestone_count: u32) -> Result<(), BoundlessError> {
        caller.require_auth();
        let mut project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        if project.creator != caller {
            return Err(BoundlessError::Unauthorized);
        }
        project.milestone_count = new_milestone_count;
        env.storage().persistent().set(&DataKey::Project(project_id.clone()), &project);
        Ok(())
    }
    fn close_project(env: Env, project_id: String, creator: Address) -> Result<(), BoundlessError> {
        creator.require_auth();
        let mut project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        if project.creator != creator {
            return Err(BoundlessError::Unauthorized);
        }
        if project.status != ProjectStatus::Voting {
            return Err(BoundlessError::Unauthorized);
        }
        project.is_closed = true;
        env.storage().persistent().set(&DataKey::Project(project_id.clone()), &project);
        Ok(())
    }
    fn get_project_status(env: Env, project_id: String) -> Result<ProjectStatus, BoundlessError> {
        let project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        Ok(project.status)
    }
    fn list_projects(env: Env) -> Result<Vec<String>, BoundlessError> {
        let projects = env.storage().persistent().get(&DataKey::Projects).unwrap_or_else(|| Vec::new(&env));
        Ok(projects)
    }
    fn get_project_stats(env: Env, project_id: String) -> Result<(u64, u64, u32), BoundlessError> {
        let project: Project = env.storage().persistent().get(&DataKey::Project(project_id.clone())).ok_or(BoundlessError::NotFound)?;
        Ok((project.funding_target, project.total_funded, project.milestone_count))
    }
}
