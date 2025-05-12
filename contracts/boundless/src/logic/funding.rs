use crate::{
    datatypes::{
        BackerContribution, BoundlessError, DataKey, Project, ProjectFundedEvent, ProjectStatus,
        RefundProcessedEvent,
    },
    interface::FundingOperations,
    BoundlessContract, BoundlessContractArgs, BoundlessContractClient,
};
use soroban_sdk::{contractimpl, token::TokenClient, Address, Env, String, Vec};

#[contractimpl]
impl FundingOperations for BoundlessContract {
    fn fund_project(
        env: Env,
        project_id: String,
        amount: i128,
        funder: Address,
        token_contract: Address,
    ) -> Result<(), BoundlessError> {
        funder.require_auth();

        if amount <= 0 {
            return Err(BoundlessError::InsufficientFunds);
        }

        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        if project.status != ProjectStatus::Funding {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.is_closed {
            return Err(BoundlessError::ProjectClosed);
        }

        let current_time = env.ledger().timestamp();

        if current_time > project.funding_deadline {
            return Err(BoundlessError::FundingPeriodEnded);
        }

        // Validate that the token contract is whitelisted/valid
        let whitelisted_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::WhitelistedTokens)
            .unwrap_or(Vec::new(&env));
        if !whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::InvalidTokenContract);
        }

        let token_client = TokenClient::new(&env, &token_contract);
        token_client.transfer(&funder, &env.current_contract_address(), &amount);

        let amount_u64 = amount as u64;
        project.total_funded += amount_u64;

        // Update the backers list
        let mut found = false;
        for mut backer_entry in project.backers.iter() {
            if backer_entry.0 == funder {
                backer_entry.1 += amount_u64;
                found = true;
                break;
            }
        }

        if !found {
            project.backers.push_back((funder.clone(), amount_u64));
        }

        // Store the backer contribution
        let backer_contribution = BackerContribution {
            backer: funder.clone(),
            amount: amount_u64,
            timestamp: current_time,
        };

        let backers_key = DataKey::Backers(project_id.clone());
        let mut backers: Vec<BackerContribution> = env
            .storage()
            .persistent()
            .get(&backers_key)
            .unwrap_or(Vec::new(&env));
        backers.push_back(backer_contribution);

        env.storage().persistent().set(&backers_key, &backers);

        if project.total_funded >= project.funding_target {
            project.status = ProjectStatus::Funded;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);

        env.events().publish(
            ("project", "funded"),
            ProjectFundedEvent {
                project_id: project_id.clone(),
                total_funded: project.total_funded,
            },
        );

        Ok(())
    }

    fn refund(env: Env, project_id: String, token_contract: Address) -> Result<(), BoundlessError> {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        if project.status != ProjectStatus::Failed && !project.is_closed {
            return Err(BoundlessError::InvalidOperation);
        }

        if project.refund_processed {
            return Err(BoundlessError::RefundAlreadyProcessed);
        }

        let whitelisted_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::WhitelistedTokens)
            .unwrap_or(Vec::new(&env));

        if !whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::InvalidTokenContract);
        }

        let mut refunded_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::RefundedTokens)
            .unwrap_or(Vec::new(&env));

        if refunded_tokens.contains(&token_contract) {
            return Err(BoundlessError::RefundAlreadyProcessed);
        }

        let contract_address = env.current_contract_address();
        let token_client = TokenClient::new(&env, &token_contract);

        let balance = token_client.balance(&contract_address);
        if balance < project.total_funded as i128 {
            return Err(BoundlessError::InsufficientFunds);
        }

        // Process refunds for each backer
        for (backer, amount) in project.backers.iter() {
            let amount_i128 = amount as i128;
            match token_client.try_transfer(&contract_address, &backer, &amount_i128) {
                Ok(_) => {
                    env.events().publish(
                        ("project", "refund"),
                        RefundProcessedEvent {
                            project_id: project_id.clone(),
                            backer: backer.clone(),
                            amount,
                        },
                    );
                }
                Err(_e) => {
                    env.events().publish(
                        ("project", "refund_failed"),
                        (project_id.clone(), backer.clone(), amount),
                    );
                }
            }
        }

        refunded_tokens.push_back(token_contract);

        env.storage()
            .persistent()
            .set(&DataKey::RefundedTokens, &refunded_tokens);

        if refunded_tokens.len() == whitelisted_tokens.len() {
            project.refund_processed = true;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        Ok(())
    }

    fn get_project_funding(env: Env, project_id: String) -> Result<(u64, u64), BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        Ok((project.total_funded, project.funding_target))
    }

    fn get_backer_contribution(
        env: Env,
        project_id: String,
        backer: Address,
    ) -> Result<u64, BoundlessError> {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .ok_or(BoundlessError::NotFound)?;

        for (addr, amount) in project.backers.iter() {
            if addr == backer {
                return Ok(amount);
            }
        }

        Ok(0)
    }

    fn whitelist_token_contract(
        env: Env,
        admin: Address,
        token_contract: Address,
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

        let mut whitelisted_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::WhitelistedTokens)
            .unwrap_or(Vec::new(&env));

        if whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::AlreadyWhitelisted);
        }

        whitelisted_tokens.push_back(token_contract);
        env.storage()
            .persistent()
            .set(&DataKey::WhitelistedTokens, &whitelisted_tokens);

        Ok(())
    }
}
