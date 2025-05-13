use crate::{
    datatypes::{BoundlessError, DataKey, Milestone, MilestoneStatus, Project, ProjectStatus},
    interface::MilestoneOperations,
    BoundlessContract, BoundlessContractArgs, BoundlessContractClient,
};
use soroban_sdk::{contractimpl, symbol_short, Address, Env, String, Vec};

#[contractimpl]
impl MilestoneOperations for BoundlessContract {
    fn release_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError> {
        admin.require_auth();
        let admin_address: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(BoundlessError::NotFound)?;
        if admin_address != admin {
            return Err(BoundlessError::Unauthorized);
        }
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        if project.status == ProjectStatus::Voting || project.status == ProjectStatus::Funding {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.creator == admin {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.milestones.len() <= milestone_number {
            return Err(BoundlessError::InvalidOperation);
        }

        let milestone = project.milestones.get_unchecked(milestone_number);
        if milestone.status != MilestoneStatus::Pending {
            return Err(BoundlessError::InvalidOperation);
        }

        // Create a new milestone with updated status
        let updated_milestone = Milestone {
            status: MilestoneStatus::Released,
            ..milestone
        };

        // Replace the milestone in the vector
        project.milestones.set(milestone_number, updated_milestone);

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
        env.events().publish(
            (
                DataKey::Project(project_id.clone()),
                symbol_short!("released"),
            ),
            milestone_number,
        );
        Ok(())
    }
    fn approve_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError> {
        admin.require_auth();
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        if project.status == ProjectStatus::Voting || project.status == ProjectStatus::Funding {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.creator == admin {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.milestones.len() <= milestone_number {
            return Err(BoundlessError::InvalidOperation);
        }

        let milestone = project.milestones.get_unchecked(milestone_number);
        if milestone.status != MilestoneStatus::Released {
            return Err(BoundlessError::InvalidOperation);
        }

        // Create a new milestone with updated status
        let updated_milestone = Milestone {
            status: MilestoneStatus::Approved,
            ..milestone
        };

        // Replace the milestone in the vector
        project.milestones.set(milestone_number, updated_milestone);

        // Add to approvals
        project
            .milestone_approvals
            .push_back((milestone_number, true));

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
        env.events().publish(
            (
                DataKey::Project(project_id.clone()),
                symbol_short!("approved"),
            ),
            milestone_number,
        );
        Ok(())
    }
    fn reject_milestone(
        env: Env,
        project_id: String,
        milestone_number: u32,
        admin: Address,
    ) -> Result<(), BoundlessError> {
        admin.require_auth();
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        if project.status == ProjectStatus::Voting || project.status == ProjectStatus::Funding {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.creator == admin {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.milestones.len() <= milestone_number {
            return Err(BoundlessError::InvalidOperation);
        }

        let milestone = project.milestones.get_unchecked(milestone_number);
        if milestone.status != MilestoneStatus::Released {
            return Err(BoundlessError::InvalidOperation);
        }

        // Create a new milestone with updated status
        let updated_milestone = Milestone {
            status: MilestoneStatus::Rejected,
            ..milestone
        };

        // Replace the milestone in the vector
        project.milestones.set(milestone_number, updated_milestone);

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
        env.events().publish(
            (
                DataKey::Project(project_id.clone()),
                symbol_short!("rejected"),
            ),
            milestone_number,
        );
        Ok(())
    }
    fn get_milestone_status(
        env: Env,
        project_id: String,
        milestone_number: u32,
    ) -> Result<MilestoneStatus, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        if project.milestones.len() <= milestone_number {
            return Err(BoundlessError::InvalidOperation);
        }
        Ok(project.milestones.get_unchecked(milestone_number).status)
    }
    fn get_project_milestones(
        env: Env,
        project_id: String,
    ) -> Result<Vec<Milestone>, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        Ok(project.milestones)
    }
}
