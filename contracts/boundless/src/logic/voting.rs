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

        let mut already_voted = false;
        let mut i = 0;
        while i < project.votes.len() {
            if project.votes.get_unchecked(i).0 == voter {
                already_voted = true;
                break;
            }
            i += 1;
        }

        if already_voted {
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

        let mut already_voted = false;
        let mut i = 0;
        while i < project.votes.len() {
            if project.votes.get_unchecked(i).0 == voter {
                already_voted = true;
                project.votes.remove(i);
                break;
            }
            i += 1;
        }

        if !already_voted {
            return Err(BoundlessError::NotVoted);
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

        let mut already_voted = false;
        let mut i = 0;
        while i < project.votes.len() {
            if project.votes.get_unchecked(i).0 == voter {
                already_voted = true;
                break;
            }
            i += 1;
        }

        Ok(already_voted)
    }
    fn get_vote(env: Env, project_id: String, voter: Address) -> Result<i32, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        let mut i = 0;
        while i < project.votes.len() {
            let vote = project.votes.get_unchecked(i);
            if vote.0 == voter {
                return Ok(vote.1);
            }
            i += 1;
        }

        Err(BoundlessError::AlreadyVoted)
    }
}
