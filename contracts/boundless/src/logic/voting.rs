use crate::{
    datatypes::{BoundlessError, DataKey, Project, ProjectStatus},
    interface::VotingOperations,
    BoundlessContract, BoundlessContractArgs, BoundlessContractClient,
};
use soroban_sdk::{contractimpl, symbol_short, Address, Env, String};

#[contractimpl]
impl VotingOperations for BoundlessContract {
    fn vote_project(
        env: Env,
        project_id: String,
        voter: Address,
        vote_value: i32,
    ) -> Result<(), BoundlessError> {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        if project.status != ProjectStatus::Voting {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.is_closed {
            return Err(BoundlessError::ProjectClosed);
        }

        if project.voting_deadline < env.ledger().timestamp() {
            return Err(BoundlessError::VotingPeriodEnded);
        }

        if project.creator == voter {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.votes.iter().any(|vote| vote.0 == voter) {
            return Err(BoundlessError::AlreadyVoted);
        }

        if vote_value != 1 && vote_value != -1 {
            return Err(BoundlessError::InvalidVote);
        }

        project.votes.push_back((voter, vote_value));

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);

        env.events().publish(
            (DataKey::Project(project_id.clone()), symbol_short!("voted")),
            project_id,
        );
        Ok(())
    }
    fn withdraw_vote(env: Env, project_id: String, voter: Address) -> Result<(), BoundlessError> {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        if project.status != ProjectStatus::Voting {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.is_closed {
            return Err(BoundlessError::ProjectClosed);
        }

        if project.voting_deadline < env.ledger().timestamp() {
            return Err(BoundlessError::VotingPeriodEnded);
        }

        if project.creator == voter {
            return Err(BoundlessError::InvalidOperation);
        }

        if !project.votes.iter().any(|vote| vote.0 == voter) {
            return Err(BoundlessError::NotVoted);
        }

        if let Some(index) = project.votes.iter().position(|vote| vote.0 == voter) {
            project.votes.remove(index as u32);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);

        env.events().publish(
            (
                DataKey::Project(project_id.clone()),
                symbol_short!("withdrawn"),
            ),
            project_id,
        );

        Ok(())
    }
    fn has_voted(env: Env, project_id: String, voter: Address) -> Result<bool, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        Ok(project.votes.iter().any(|vote| vote.0 == voter))
    }
    fn get_vote(env: Env, project_id: String, voter: Address) -> Result<i32, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;
        for vote in project.votes {
            if vote.0 == voter {
                return Ok(vote.1);
            }
        }
        return Err(BoundlessError::NotVoted);
    }
}
