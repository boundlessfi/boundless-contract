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
            .get(&DataKey::WhitelistedTokens(project_id.clone()))
            .unwrap_or(Vec::new(&env));
        if !whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::InvalidTokenContract);
        }

        let token_client = TokenClient::new(&env, &token_contract);
        token_client
            .try_transfer(&funder, &env.current_contract_address(), &amount)
            .map_err(|_| BoundlessError::TransferFailed)?
            .map_err(|_| BoundlessError::TransferFailed)?;

        let amount_u64 = amount as u64;
        project.total_funded += amount_u64;

        // Update the backers list
        let mut found = false;
        let mut updated_backers = Vec::new(&env);

        let mut i = 0;
        while i < project.backers.len() {
            let (backer_address, backer_amount, token) = project.backers.get_unchecked(i);
            if backer_address == funder && token == token_contract {
                updated_backers.push_back((
                    backer_address.clone(),
                    backer_amount + amount_u64,
                    token.clone(),
                ));
                found = true;
            } else {
                updated_backers.push_back((backer_address.clone(), backer_amount, token.clone()));
            }
            i += 1;
        }

        if !found {
            updated_backers.push_back((funder.clone(), amount_u64, token_contract.clone()));
        }
        project.backers = updated_backers;

        // Store the backer contribution
        let backer_contribution = BackerContribution {
            backer: funder.clone(),
            amount: amount_u64,
            token: token_contract.clone(),
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

        if project.backers.is_empty() {
            return Err(BoundlessError::NoBackerContributions);
        }

        if project.refund_processed {
            return Err(BoundlessError::RefundAlreadyProcessed);
        }

        let whitelisted_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::WhitelistedTokens(project_id.clone()))
            .unwrap_or(Vec::new(&env));

        if !whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::InvalidTokenContract);
        }

        let mut refunded_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::RefundedTokens(project_id.clone()))
            .unwrap_or(Vec::new(&env));

        if refunded_tokens.contains(&token_contract) {
            return Err(BoundlessError::RefundAlreadyProcessed);
        }

        let contract_address = env.current_contract_address();
        let token_client = TokenClient::new(&env, &token_contract);
        let balance = token_client
            .try_balance(&contract_address)
            .map_err(|_| BoundlessError::BalanceCheckFailed)?
            .map_err(|_| BoundlessError::BalanceCheckFailed)?;

        // Check if the contract has enough balance to refund
        let mut refund_amount = 0_u64;
        let backer_contributions: Vec<BackerContribution> = env
            .storage()
            .persistent()
            .get(&DataKey::Backers(project_id.clone()))
            .unwrap_or(Vec::new(&env));
        let mut i = 0;
        while i < backer_contributions.len() {
            let backer_contribution = backer_contributions.get_unchecked(i);
            if backer_contribution.token == token_contract {
                refund_amount += backer_contribution.amount;
            }
            i += 1;
        }

        if balance < refund_amount as i128 {
            return Err(BoundlessError::InsufficientFunds);
        }

        // Process refunds for each backer
        let mut i = 0;
        while i < backer_contributions.len() {
            let backer_contribution = backer_contributions.get_unchecked(i);
            let (backer, amount, token, _) = (
                backer_contribution.backer.clone(),
                backer_contribution.amount as i128,
                backer_contribution.token.clone(),
                backer_contribution.timestamp,
            );

            if token != token_contract {
                i += 1;
                continue;
            }

            match token_client.try_transfer(&contract_address, &backer, &amount) {
                Ok(_) => {
                    env.events().publish(
                        ("project", "refund"),
                        RefundProcessedEvent {
                            project_id: project_id.clone(),
                            backer: backer.clone(),
                            amount: amount as u64,
                        },
                    );
                }
                Err(_e) => {
                    env.events().publish(
                        ("project", "refund_failed"),
                        (project_id.clone(), backer.clone(), amount as u64),
                    );
                }
            }
            i += 1;
        }

        refunded_tokens.push_back(token_contract);

        env.storage().persistent().set(
            &DataKey::RefundedTokens(project_id.clone()),
            &refunded_tokens,
        );

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

        let mut i = 0;
        while i < project.backers.len() {
            let (addr, amount, _) = project.backers.get_unchecked(i);
            if addr == backer {
                return Ok(amount);
            }
            i += 1;
        }

        Ok(0)
    }

    fn whitelist_token_contract(
        env: Env,
        admin: Address,
        project_id: String,
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
            .get(&DataKey::WhitelistedTokens(project_id.clone()))
            .unwrap_or(Vec::new(&env));

        if whitelisted_tokens.contains(&token_contract) {
            return Err(BoundlessError::AlreadyWhitelisted);
        }

        whitelisted_tokens.push_back(token_contract);
        env.storage().persistent().set(
            &DataKey::WhitelistedTokens(project_id.clone()),
            &whitelisted_tokens,
        );

        Ok(())
    }
}
