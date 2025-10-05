use crate::datatypes::{
    Backer, BoundlessError, Campaign, CampaignCancelled, CampaignFunded, CampaignStatusUpdated,
    FundsReleased, Milestone, MilestoneStatus, Status,
};
use crate::interface::{CampaignManagement, ContractManagement};
use crate::{BoundlessContract, BoundlessContractArgs, BoundlessContractClient};
use soroban_sdk::{contractimpl, Address, Env, Symbol, Vec};

/// Campaign logic implementation
#[contractimpl]
impl CampaignManagement for BoundlessContract {
    fn create_campaign(
        env: Env,
        owner: Address,
        title: Symbol,
        description: Symbol,
        goal: i128,
        escrow_contract_id: Address,
        milestones: Vec<Milestone>,
    ) -> Result<u64, BoundlessError> {
        owner.require_auth();
        // TODO: campaign creation logic
        // - Generate unique campaign ID
        // - Create Campaign struct with Status::Active
        // - Store in persistent storage
        // - Return campaign ID
        Ok(0) // Placeholder
    }

    fn fund_campaign(
        env: Env,
        campaign_id: u64,
        backer: Address,
        amount: i128,
    ) -> Result<(), BoundlessError> {
        backer.require_auth();

        if amount <= 0 {
            return Err(BoundlessError::InvalidOperation);
        }

        let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
        let mut campaign: Campaign = env
            .storage()
            .persistent()
            .get(&campaign_key)
            .ok_or(BoundlessError::CampaignNotFound)?;

        campaign.backers.push_back(Backer {
            wallet: backer.clone(),
            amount,
        });

        env.storage().persistent().set(&campaign_key, &campaign);

        CampaignFunded {
            campaign_id,
            backer,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    fn release_funds(env: Env, campaign_id: u64, milestone_id: u64) -> Result<(), BoundlessError> {
        // Get the campaign from storage
        let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
        let mut campaign: Campaign = env
            .storage()
            .persistent()
            .get(&campaign_key)
            .ok_or(BoundlessError::CampaignNotFound)?;

        // Validate campaign status allows fund release
        match campaign.status {
            Status::Failed => return Err(BoundlessError::InvalidOperation),
            Status::Completed => return Err(BoundlessError::InvalidOperation),
            _ => {} // Allow release for Pending and Active campaigns
        }

        // Find the milestone in the campaign's milestones list
        let mut milestone_found = false;
        let mut milestone_amount = 0i128;

        for milestone in campaign.milestones.iter() {
            if milestone.id == milestone_id {
                milestone_found = true;
                milestone_amount = milestone.amount;

                // Validate milestone status allows release
                match milestone.status {
                    MilestoneStatus::Approved => {
                        // Milestone can be released
                    }
                    MilestoneStatus::Released => {
                        return Err(BoundlessError::InvalidOperation); // Already released
                    }
                    MilestoneStatus::Rejected => {
                        return Err(BoundlessError::InvalidOperation); // Cannot release rejected milestone
                    }
                    MilestoneStatus::Pending => {
                        return Err(BoundlessError::InvalidOperation); // Must be approved first
                    }
                }
                break;
            }
        }

        if !milestone_found {
            return Err(BoundlessError::MilestoneNotFound);
        }

        // Update the milestone status to Released
        let mut updated_milestones = Vec::new(&env);
        for milestone in campaign.milestones.iter() {
            if milestone.id == milestone_id {
                let mut updated_milestone = milestone.clone();
                updated_milestone.status = MilestoneStatus::Released;
                updated_milestones.push_back(updated_milestone);
            } else {
                updated_milestones.push_back(milestone.clone());
            }
        }
        campaign.milestones = updated_milestones;

        // Store the updated campaign
        env.storage().persistent().set(&campaign_key, &campaign);

        // Emit the release event
        FundsReleased {
            campaign_id,
            milestone_id,
            amount: milestone_amount,
            releaser: env.current_contract_address(),
        }
        .publish(&env);

        Ok(())
    }

    fn get_campaign(env: Env, campaign_id: u64) -> Result<Campaign, BoundlessError> {
        // TODO: get campaign logic
        // - Retrieve campaign from storage
        // - Return campaign struct
        Err(BoundlessError::CampaignNotFound) // Placeholder
    }

    fn complete_campaign(env: Env, campaign_id: u64, admin: Address) -> Result<(), BoundlessError> {
        // TODO: complete campaign logic
        // - Verify admin authorization
        // - Get campaign from storage
        // - Update status to Completed
        // - Store updated campaign
        // - Emit completion event
        Ok(())
    }

    fn cancel_campaign(env: Env, campaign_id: u64, admin: Address) -> Result<(), BoundlessError> {
        admin.require_auth();
        let contract_admin = <BoundlessContract as ContractManagement>::get_admin(&env);
        if admin != contract_admin {
            return Err(BoundlessError::Unauthorized);
        }

        let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
        let mut campaign: Campaign = env
            .storage()
            .persistent()
            .get(&campaign_key)
            .ok_or(BoundlessError::CampaignNotFound)?;

        match campaign.status {
            Status::Completed => return Err(BoundlessError::InvalidOperation),
            Status::Failed => return Err(BoundlessError::InvalidOperation),
            _ => {}
        }

        campaign.status = Status::Failed;
        env.storage().persistent().set(&campaign_key, &campaign);
        // env.events().publish(
        //     (Symbol::new(&env, "campaign"), Symbol::new(&env, "stop")),
        //     (campaign_id, admin),
        // );
        CampaignCancelled { campaign_id, admin }.publish(&env);

        Ok(())
    }

    fn update_campaign_status(
        env: Env,
        campaign_id: u64,
        status: Status,
        admin: Address,
    ) -> Result<(), BoundlessError> {
        admin.require_auth();
        let contract_admin = <BoundlessContract as ContractManagement>::get_admin(&env);
        if admin != contract_admin {
            return Err(BoundlessError::Unauthorized);
        }

        let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
        let mut campaign: Campaign = env
            .storage()
            .persistent()
            .get(&campaign_key)
            .ok_or(BoundlessError::CampaignNotFound)?;

        campaign.status = status.clone();
        env.storage().persistent().set(&campaign_key, &campaign);
        // env.events().publish(
        //     (Symbol::new(&env, "campaign"), Symbol::new(&env, "status_update")),
        //     (campaign_id, status, admin),
        // );
        CampaignStatusUpdated {
            campaign_id,
            status,
            admin,
        }
        .publish(&env);

        Ok(())
    }

    fn get_campaign_backers(env: Env, campaign_id: u64) -> Result<Vec<Backer>, BoundlessError> {
        // TODO: get campaign backers logic
        // - Get campaign from storage
        // - Return backers list
        Ok(Vec::new(&env)) // Placeholder
    }
}
