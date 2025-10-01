#![cfg(test)]

use crate::{
    datatypes::{Campaign, Status},
    BoundlessContract, BoundlessContractClient,
};
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, Symbol, Vec,
};

extern crate std;
mod boundless {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/boundless.wasm");
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
    env.storage().persistent().set(&campaign_key, &campaign);

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
    env.storage().persistent().set(&campaign_key, &campaign);
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
    env.storage().persistent().set(&campaign_key, &campaign);
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
