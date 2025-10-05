#![cfg(test)]

use crate::{
    datatypes::{Campaign, DataKey, Status},
    BoundlessContract, BoundlessContractClient,
};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};

#[test]
fn test_fund_campaign_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

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

    let campaign_key = DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    let backer = Address::generate(&env);
    let funding_amount = 100i128;

    contract.fund_campaign(&campaign_id, &backer, &funding_amount);

    let updated_campaign: Campaign = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&campaign_key).unwrap()
    });

    assert_eq!(updated_campaign.backers.len(), 1);
    assert_eq!(updated_campaign.backers.get(0).unwrap().wallet, backer);
    assert_eq!(
        updated_campaign.backers.get(0).unwrap().amount,
        funding_amount
    );
}

#[test]
fn test_fund_campaign_multiple_backers() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(BoundlessContract, ());
    let contract = BoundlessContractClient::new(&env, &contract_id);

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

    let campaign_key = DataKey::Campaign(campaign_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&campaign_key, &campaign);
    });

    let backer1 = Address::generate(&env);
    let backer2 = Address::generate(&env);
    let backer3 = Address::generate(&env);

    contract.fund_campaign(&campaign_id, &backer1, &100i128);
    contract.fund_campaign(&campaign_id, &backer2, &200i128);
    contract.fund_campaign(&campaign_id, &backer3, &300i128);

    let updated_campaign: Campaign = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&campaign_key).unwrap()
    });

    assert_eq!(updated_campaign.backers.len(), 3);
}
