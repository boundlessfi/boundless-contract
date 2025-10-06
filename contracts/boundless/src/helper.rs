use crate::datatypes::{BoundlessError, CampaignCompleted, DataKey};
use soroban_sdk::{Env, Address};

// Helper function to check platform admin
pub fn is_platform_admin(env: &Env, address: &Address) -> Result<bool, BoundlessError> {
    let admin: Address = env
        .storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(BoundlessError::Unauthorized)?;

    Ok(&admin == address)
}

// Helper function to emit event
pub fn emit_campaign_completed_event(env: &Env, campaign_id: u64, completed_by: Address) {
    let event_data = CampaignCompleted {
        campaign_id,
        completed_by,
        completed_at: env.ledger().timestamp(),
    };

    event_data.publish(env);
}
