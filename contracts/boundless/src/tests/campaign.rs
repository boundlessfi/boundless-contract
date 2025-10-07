#![cfg(test)]

use crate::{
    datatypes::{Campaign, EntityType, Milestone, MilestoneStatus, Status},
    BoundlessContract, BoundlessContractClient,
};
use soroban_sdk::{log, testutils::Address as _, Address, Env, Symbol, Vec};

extern crate std;
mod boundless {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/boundless.wasm");
}

#[test]
fn test_get_campaign_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner: Address = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    let campaign = contract.get_campaign(&campaign_id);
    assert_eq!(campaign.funding_goal, funding_goal);
    assert_eq!(campaign.title, title);
    assert_eq!(campaign.status, Status::Active);
}

#[test]
#[should_panic]
fn test_get_campaign_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let _ = contract.get_campaign(&campaign_id);
}

#[test]
fn test_cancel_campaign_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner: Address = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    contract.cancel_campaign(&campaign_id, &admin);

    let updated_campaign: Campaign = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&campaign_key).unwrap()
    });
    assert_eq!(updated_campaign.status, Status::Failed);
}

#[test]
#[should_panic]
fn test_cancel_campaign_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    contract.cancel_campaign(&campaign_id, &unauthorized_user);
}

#[test]
#[should_panic]
fn test_cancel_campaign_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);
    contract.cancel_campaign(&999u64, &admin);
}

#[test]
#[should_panic]
fn test_cancel_campaign_already_completed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Completed,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });
    contract.cancel_campaign(&campaign_id, &admin);
}

#[test]
#[should_panic]
fn test_cancel_campaign_already_failed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Failed,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });
    contract.cancel_campaign(&campaign_id, &admin);
}

#[test]
fn test_cancel_campaign_pending_status() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env);
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Pending,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    contract.cancel_campaign(&campaign_id, &admin);

    let updated_campaign: Campaign = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&campaign_key).unwrap()
    });
    assert_eq!(updated_campaign.status, Status::Failed);
}

#[test]
fn test_release_funds_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Approved status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Approved,
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Release funds
    contract.release_funds(&campaign_id, &milestone_id);

    // Verify milestone status was updated to Released
    let updated_campaign: Campaign = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&campaign_key).unwrap()
    });

    let released_milestone = updated_campaign.milestones.get(0).unwrap();
    assert_eq!(released_milestone.status, MilestoneStatus::Released);
}

#[test]
#[should_panic]
fn test_release_funds_campaign_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    // Try to release funds for non-existent campaign
    contract.release_funds(&999u64, &1u64);
}

#[test]
#[should_panic]
fn test_release_funds_milestone_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let milestones = Vec::new(&env); // Empty milestones
    let backers = Vec::new(&env);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones,
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds for non-existent milestone
    contract.release_funds(&campaign_id, &999u64);
}

#[test]
#[should_panic]
fn test_release_funds_campaign_failed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Approved status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Approved,
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Failed, // Campaign is failed
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds from failed campaign
    contract.release_funds(&campaign_id, &milestone_id);
}

#[test]
#[should_panic]
fn test_release_funds_campaign_completed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Approved status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Approved,
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Completed, // Campaign is completed
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds from completed campaign
    contract.release_funds(&campaign_id, &milestone_id);
}

#[test]
#[should_panic]
fn test_release_funds_milestone_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Pending status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Pending, // Milestone is pending
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds from pending milestone
    contract.release_funds(&campaign_id, &milestone_id);
}

#[test]
#[should_panic]
fn test_release_funds_milestone_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Rejected status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Rejected, // Milestone is rejected
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds from rejected milestone
    contract.release_funds(&campaign_id, &milestone_id);
}

#[test]
#[should_panic]
fn test_release_funds_milestone_already_released() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

    contract.initialize(&admin);

    let campaign_id = 1u64;
    let milestone_id = 1u64;
    let owner = Address::generate(&env);
    let title = Symbol::new(&env, "TestCampaign");
    let description = Symbol::new(&env, "TestDescription");
    let funding_goal = 1000i128;
    let escrow_contract_id = Address::generate(&env);
    let backers = Vec::new(&env);

    // Create a milestone with Released status
    let mut milestones = Vec::new(&env);
    let milestone = Milestone {
        id: milestone_id,
        entity_id: campaign_id,
        entity_type: EntityType::Campaign,
        description: Symbol::new(&env, "TestMilestone"),
        amount: 500i128,
        status: MilestoneStatus::Released, // Milestone is already released
    };
    milestones.push_back(milestone);

    let campaign = Campaign {
        id: campaign_id,
        owner: owner.clone(),
        title: title.clone(),
        description: description.clone(),
        funding_goal,
        escrow_contract_id: escrow_contract_id.clone(),
        milestones: milestones.clone(),
        backers,
        status: Status::Active,
    };

    let campaign_key = crate::datatypes::DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    // Try to release funds from already released milestone
    contract.release_funds(&campaign_id, &milestone_id);
}
