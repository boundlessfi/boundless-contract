#![cfg(test)]

use crate::{
    datatypes::{BoundlessError, Project, ProjectStatus, FUNDING_PERIOD_LEDGERS, VOTING_PERIOD_LEDGERS}, BoundlessContract, BoundlessContractClient
};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, BytesN, Env, String};

#[test]
fn test_vote_project() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let voter = Address::generate(&env);
    let vote_value = 1;
    client.vote_project(&project_id, &voter, &vote_value);
    let project = client.get_project(&project_id);
    assert_eq!(project.votes.len(), 1);
    assert_eq!(project.votes.get_unchecked(0).0, voter);
    assert_eq!(project.votes.get_unchecked(0).1, vote_value);
}

#[test]
#[should_panic]
fn test_vote_project_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
}

#[test]
#[should_panic]
fn test_vote_project_invalid_vote_value() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 2;
    client.vote_project(&project_id, &creator, &vote_value);
}

#[test]
#[should_panic]
fn test_vote_project_project_closed() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
    client.close_project(&project_id, &creator);
    client.vote_project(&project_id, &creator, &vote_value);
}

#[test]
#[should_panic]
fn test_vote_project_funding_period_ended() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
    env.ledger().set_timestamp(FUNDING_PERIOD_LEDGERS as u64);
    client.vote_project(&project_id, &creator, &vote_value);
}

#[test]
fn test_withdraw_vote() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let voter = Address::generate(&env);
    let vote_value = 1;
    client.vote_project(&project_id, &voter, &vote_value);
    client.withdraw_vote(&project_id, &voter);
    let project = client.get_project(&project_id);
    assert_eq!(project.votes.len(), 0);
}

#[test]
#[should_panic]
fn test_withdraw_vote_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let voter = Address::generate(&env);
    let vote_value = 1;
    client.vote_project(&project_id, &voter, &vote_value);
    client.withdraw_vote(&project_id, &creator);
}

#[test]
#[should_panic]
fn test_withdraw_vote_project_closed() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
    client.close_project(&project_id, &creator);
    client.withdraw_vote(&project_id, &creator);
}

#[test]
#[should_panic]
fn test_withdraw_vote_voting_period_ended() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
    env.ledger().set_timestamp(VOTING_PERIOD_LEDGERS as u64);
    client.withdraw_vote(&project_id, &creator);
}

#[test]
#[should_panic]
fn test_withdraw_vote_already_voted() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let vote_value = 1;
    client.vote_project(&project_id, &creator, &vote_value);
    client.withdraw_vote(&project_id, &creator);
    client.withdraw_vote(&project_id, &creator);
}

#[test]
fn test_has_voted() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let voter = Address::generate(&env);
    let vote_value = 1;
    client.vote_project(&project_id, &voter, &vote_value);
    assert_eq!(client.has_voted(&project_id, &voter), true);
}

#[test]
fn test_get_vote() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    env.mock_all_auths();
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    let voter = Address::generate(&env);
    let vote_value = 1;
    client.vote_project(&project_id, &voter, &vote_value);
    assert_eq!(client.get_vote(&project_id, &voter), vote_value);
}