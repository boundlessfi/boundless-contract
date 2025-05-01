#![cfg(test)]

use crate::{
    datatypes::{BackerContribution, BoundlessError, ProjectStatus},
    interface::{BoundlessContract, ContractManagement, FundingOperations, ProjectManagement},
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token::{Client as TokenClient, StellarAssetClient, TokenMetadata},
    Address, Env, IntoVal, String, Vec,
};

fn create_token_contract(env: &Env) -> (Address, TokenClient) {
    let admin = Address::random(env);
    let token_address = env.register_stellar_asset_contract_v2(admin.clone());
    let token = TokenClient::new(env, &token_address);
    let asset_client = StellarAssetClient::new(env, &token_address);
    
    // Set token metadata
    let metadata = TokenMetadata {
        name: "Test Token".into_val(env),
        symbol: "TEST".into_val(env),
        decimals: 7,
    };
    asset_client.set_metadata(&metadata);
    
    (token_address, token)
}

fn setup_test() -> (Env, Address, Address, TokenClient) {
    let env = Env::default();
    env.ledger().set(Ledger {
        timestamp: 12345,
        protocol_version: 20,
        sequence_number: 100,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 999999,
        min_persistent_entry_ttl: 999999,
        max_entry_ttl: 9999999,
    });

    let admin = Address::random(&env);
    let contract_id = env.register_contract(None, BoundlessContract);
    
    let client = BoundlessContract::new(&env, &contract_id);
    client.initialize(env.clone(), admin.clone()).unwrap();
    
    let (token_address, token) = create_token_contract(&env);
    
    // Mint some tokens for testing
    let user = Address::random(&env);
    token.mint(&user, &1000000);
    
    (env, contract_id, user, token)
}

#[test]
fn test_fund_project_success() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Fund the project
    let funding_amount = 200000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Verify the funding was recorded
    let (total_funded, target) = client.get_project_funding(env.clone(), project_id.clone()).unwrap();
    assert_eq!(total_funded, funding_amount as u64);
    assert_eq!(target, funding_target);
    
    // Verify backer contribution
    let contribution = client.get_backer_contribution(env.clone(), project_id.clone(), user.clone()).unwrap();
    assert_eq!(contribution, funding_amount as u64);
}

#[test]
fn test_fund_project_multiple_backers() {
    let (env, contract_id, user1, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user1.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Create second user
    let user2 = Address::random(&env);
    token.mint(&user2, &1000000);
    
    // Fund the project from user1
    let funding_amount1 = 200000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount1,
        user1.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Fund the project from user2
    let funding_amount2 = 300000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount2,
        user2.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Verify total funding
    let (total_funded, _) = client.get_project_funding(env.clone(), project_id.clone()).unwrap();
    assert_eq!(total_funded, (funding_amount1 + funding_amount2) as u64);
    
    // Verify individual contributions
    let contribution1 = client.get_backer_contribution(env.clone(), project_id.clone(), user1.clone()).unwrap();
    let contribution2 = client.get_backer_contribution(env.clone(), project_id.clone(), user2.clone()).unwrap();
    assert_eq!(contribution1, funding_amount1 as u64);
    assert_eq!(contribution2, funding_amount2 as u64);
    
    // Verify project is now fully funded
    let project = client.get_project(env.clone(), project_id.clone()).unwrap();
    assert_eq!(project.status, ProjectStatus::Funded);
}

#[test]
fn test_fund_project_multiple_contributions() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Fund the project twice from the same user
    let funding_amount1 = 200000_i128;
    let funding_amount2 = 100000_i128;
    
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount1,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount2,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Verify total funding
    let (total_funded, _) = client.get_project_funding(env.clone(), project_id.clone()).unwrap();
    assert_eq!(total_funded, (funding_amount1 + funding_amount2) as u64);
    
    // Verify backer contribution is cumulative
    let contribution = client.get_backer_contribution(env.clone(), project_id.clone(), user.clone()).unwrap();
    assert_eq!(contribution, (funding_amount1 + funding_amount2) as u64);
}

#[test]
fn test_fund_project_invalid_status() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create and fully fund a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Fund the project fully
    let funding_amount = 500000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Try to fund again when project is already funded
    let result = client.fund_project(
        env.clone(),
        project_id.clone(),
        100000,
        user.clone(),
        token.address.clone(),
    );
    
    assert_eq!(result, Err(BoundlessError::InvalidOperation));
}

#[test]
fn test_fund_project_past_deadline() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Advance ledger to past the funding deadline
    let project = client.get_project(env.clone(), project_id.clone()).unwrap();
    env.ledger().set(Ledger {
        sequence_number: project.funding_deadline + 1,
        ..env.ledger().get()
    });
    
    // Try to fund the project after deadline
    let result = client.fund_project(
        env.clone(),
        project_id.clone(),
        100000,
        user.clone(),
        token.address.clone(),
    );
    
    assert_eq!(result, Err(BoundlessError::FundingPeriodEnded));
}

#[test]
fn test_refund_closed_project() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Fund the project
    let funding_amount = 200000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Close the project
    client.close_project(env.clone(), project_id.clone(), user.clone()).unwrap();
    
    // Process refunds
    client.refund(env.clone(), project_id.clone(), token.address.clone()).unwrap();
    
    // Check refunds were processed
    let project = client.get_project(env.clone(), project_id.clone()).unwrap();
    assert!(project.refund_processed);
    
    // Verify events
    let events = env.events().all();
    let refund_events: Vec<_> = events
        .iter()
        .filter(|event| event.topics.get(0).unwrap() == "project" && event.topics.get(1).unwrap() == "refund")
        .collect(&env);
    
    assert_eq!(refund_events.len(), 1);
}

#[test]
fn test_refund_already_processed() {
    let (env, contract_id, user, token) = setup_test();
    let client = BoundlessContract::new(&env, &contract_id);
    
    // Create a project
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 3_u32;
    
    client.create_project(
        env.clone(),
        project_id.clone(),
        user.clone(),
        metadata_uri,
        funding_target,
        milestone_count,
    ).unwrap();
    
    // Fund the project
    let funding_amount = 200000_i128;
    client.fund_project(
        env.clone(),
        project_id.clone(),
        funding_amount,
        user.clone(),
        token.address.clone(),
    ).unwrap();
    
    // Close the project
    client.close_project(env.clone(), project_id.clone(), user.clone()).unwrap();
    
    // Process refunds
    client.refund(env.clone(), project_id.clone(), token.address.clone()).unwrap();
    
    // Try to process refunds again
    let result = client.refund(env.clone(), project_id.clone(), token.address.clone());
    assert_eq!(result, Err(BoundlessError::RefundAlreadyProcessed));
}
