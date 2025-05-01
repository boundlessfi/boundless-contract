use soroban_sdk::{token, Address, Env, String, Vec};

use crate::datatypes::{
    BackerContribution, BoundlessError, DataKey, ProjectFundedEvent, ProjectStatus,
    RefundProcessedEvent, FUNDING_PERIOD_LEDGERS, PROJECTS_BUMP_AMOUNT,
};
use crate::interface::{BoundlessContract, FundingOperations};
use crate::logic::project::ProjectManagement::get_project;

impl FundingOperations for BoundlessContract {
    fn fund_project(
        env: Env,
        project_id: String,
        amount: i128,
        funder: Address,
        token_contract: Address,
    ) -> Result<(), BoundlessError> {
        // Verify the funder is the caller
        funder.require_auth();

        // Validate amount
        if amount <= 0 {
            return Err(BoundlessError::InsufficientFunds);
        }

        // Load the project
        let mut project = get_project(&env, &project_id)?;

        // Verify project is in funding phase
        if project.status != ProjectStatus::Funding {
            return Err(BoundlessError::InvalidOperation);
        }

        // Verify project is not closed
        if project.is_closed {
            return Err(BoundlessError::ProjectClosed);
        }

        // Check funding deadline
        let current_ledger = env.ledger().sequence();
        if current_ledger > project.funding_deadline {
            return Err(BoundlessError::FundingPeriodEnded);
        }

        // Transfer tokens from funder to contract
        let token_client = token::Client::new(&env, &token_contract);
        token_client.transfer(&funder, &env.current_contract_address(), &amount);

        // Convert i128 to u64 for project tracking
        let amount_u64 = amount as u64;

        // Update project's total funding
        project.total_funded += amount_u64;

        // Update or add backer contribution
        let mut found = false;
        for backer_entry in project.backers.iter_mut() {
            if backer_entry.0 == funder {
                backer_entry.1 += amount_u64;
                found = true;
                break;
            }
        }

        if !found {
            project.backers.push((funder.clone(), amount_u64));
        }

        // Store backer contribution details
        let backer_contribution = BackerContribution {
            backer: funder.clone(),
            amount: amount_u64,
            timestamp: current_ledger,
        };
        
        let backers_key = DataKey::Backers(project_id.clone());
        let mut backers: Vec<BackerContribution> = env.storage().temporary().get(&backers_key).unwrap_or(Vec::new(&env));
        backers.push(backer_contribution);
        env.storage().temporary().set(&backers_key, &backers);
        env.storage().temporary().bump(&backers_key, PROJECTS_BUMP_AMOUNT);

        // Check if the funding target is reached
        if project.total_funded >= project.funding_target {
            project.status = ProjectStatus::Funded;
            
            // Emit project funded event
            env.events().publish(
                ("project", "funded"),
                ProjectFundedEvent {
                    project_id: project_id.clone(),
                    total_funded: project.total_funded,
                },
            );
        }

        // Save the updated project
        env.storage().temporary().set(&DataKey::Project(project_id), &project);
        env.storage().temporary().bump(&DataKey::Project(project_id), PROJECTS_BUMP_AMOUNT);

        Ok(())
    }

    fn refund(
        env: Env, 
        project_id: String,
        token_contract: Address
    ) -> Result<(), BoundlessError> {
        // Load the project
        let mut project = get_project(&env, &project_id)?;

        // Verify the project status allows refunds (either Failed or Closed)
        if project.status != ProjectStatus::Failed && !project.is_closed {
            return Err(BoundlessError::InvalidOperation);
        }

        // Check if refunds already processed
        if project.refund_processed {
            return Err(BoundlessError::RefundAlreadyProcessed);
        }

        // Get the contract address for token transfers
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token_contract);

        // Process refunds for each backer
        for (backer, amount) in project.backers.iter() {
            let amount_i128 = *amount as i128;
            
            // Transfer tokens back to the backer
            token_client.transfer(&contract_address, backer, &amount_i128);
            
            // Emit refund event
            env.events().publish(
                ("project", "refund"),
                RefundProcessedEvent {
                    project_id: project_id.clone(),
                    backer: backer.clone(),
                    amount: *amount,
                },
            );
        }

        // Mark refunds as processed
        project.refund_processed = true;
        
        // Save the updated project
        env.storage().temporary().set(&DataKey::Project(project_id), &project);
        env.storage().temporary().bump(&DataKey::Project(project_id), PROJECTS_BUMP_AMOUNT);

        Ok(())
    }

    fn get_project_funding(
        env: Env,
        project_id: String
    ) -> Result<(u64, u64), BoundlessError> {
        // Load the project
        let project = get_project(&env, &project_id)?;
        
        // Return (total_funded, funding_target)
        Ok((project.total_funded, project.funding_target))
    }

    fn get_backer_contribution(
        env: Env,
        project_id: String,
        backer: Address
    ) -> Result<u64, BoundlessError> {
        // Load the project
        let project = get_project(&env, &project_id)?;
        
        // Find the backer's contribution
        for (addr, amount) in project.backers.iter() {
            if *addr == backer {
                return Ok(*amount);
            }
        }
        
        // Backer not found, return 0
        Ok(0)
    }
}
