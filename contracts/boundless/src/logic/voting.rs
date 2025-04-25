use crate::{
    datatypes::{BoundlessError, DataKey, Project, ProjectStatus, VOTING_PERIOD_LEDGERS},
    interface::{ProjectManagement, VotingOperations},
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
        // let mut project: Project = env
        //     .storage()
        //     .persistent()
        //     .get(&DataKey::Project(project_id.clone()))
        //     .ok_or(BoundlessError::NotFound)?;
        // if project.status != ProjectStatus::Voting {
        //     return Err(BoundlessError::InvalidOperation);
        // }
        // if vote_value != 1 && vote_value != -1 {
        //     return Err(BoundlessError::InvalidVote);
        // }
        // if project.creator == voter {
        //     return Err(BoundlessError::InvalidOperation);
        // }
        // if project.is_closed {
        //     return Err(BoundlessError::InvalidOperation);
        // }
        // if project.voting_deadline < env.ledger().timestamp() {
        //     return Err(BoundlessError::VotingPeriodEnded);
        // }
        // if project.votes.iter().any(|(voter, _)| voter == voter) {
        //     return Err(BoundlessError::AlreadyVoted);
        // }
        // project.votes.push_back((voter, vote_value));

        // let total_votes = project.votes.len();
        // let positive_votes = project.votes.iter().filter(|(_, vote)| *vote == 1).count();
        // let voting_threshold: u32 = 1; // Define your threshold here

        // if (total_votes as u32) >= voting_threshold
        //     && positive_votes > (total_votes / 2).try_into().unwrap()
        // {
        //     // Change project status to funding().try_into().unwrap()
        //     project.status = ProjectStatus::Funding;
        //     project.funding_deadline = env.ledger().timestamp() + VOTING_PERIOD_LEDGERS as u64;

        //     // Publish event for status change
        //     env.events().publish(
        //         (
        //             DataKey::Project(project_id.clone()),
        //             symbol_short!("status"),
        //         ),
        //         (project.project_id.clone(), ProjectStatus::Funding),
        //     );
        // }

        // env.storage()
        //     .persistent()
        //     .set(&DataKey::Project(project_id.clone()), &project);
        // env.events().publish(
        //     (DataKey::Project(project_id.clone()), symbol_short!("voted")),
        //     project.project_id,
        // );

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
        if project.creator == voter {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.is_closed {
            return Err(BoundlessError::InvalidOperation);
        }
        if project.voting_deadline < env.ledger().timestamp() {
            return Err(BoundlessError::VotingPeriodEnded);
        }

        // Find and remove the vote by the voter
        let mut i = 0;
        while i < project.votes.len() {
            if project.votes.get_unchecked(i).0 == voter {
                project.votes.remove(i);
                break;
            }
            i += 1;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
        env.events().publish(
            (
                DataKey::Project(project_id.clone()),
                symbol_short!("withdrawn"),
            ),
            project.project_id,
        );
        Ok(())
    }
    fn has_voted(env: Env, project_id: String, voter: Address) -> Result<bool, BoundlessError> {
        // let project: Project = env
        //     .storage()
        //     .persistent()
        //     .get(&DataKey::Project(project_id.clone()))
        //     .ok_or(BoundlessError::NotFound)?;
        // for vote in project.votes.iter() {
        //     if vote.0 == voter {
        //         return Ok(true);
        //     }
        // }
        Ok(false)
    }
    fn get_vote(env: Env, project_id: String, voter: Address) -> Result<i32, BoundlessError> {
        // let project: Project = env
        //     .storage()
        //     .persistent()
        //     .get(&DataKey::Project(project_id.clone()))
        //     .ok_or(BoundlessError::NotFound)?;
        // for vote in project.votes.iter() {
        //     if vote.0 == voter {
        //         return Ok(vote.1);
        //     }
        // }
        Ok(0)
    }
}
