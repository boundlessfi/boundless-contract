#![cfg(test)]

use crate::{
    datatypes::{BoundlessError, Project, ProjectStatus}, BoundlessContract, BoundlessContractClient
};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

#[test]
fn test_create_project() {
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
    let project = client.get_project(&project_id);
    assert_eq!(project.project_id, project_id);
    assert_eq!(project.creator, creator);
    assert_eq!(project.metadata_uri, metadata_uri);
    assert_eq!(project.funding_target, funding_target);
    assert_eq!(project.milestone_count, milestone_count);
    assert_eq!(project.status, ProjectStatus::Voting);
    // client.create_project(project_id.clone(), creator, metadata_uri, funding_target, milestone_count));

    // let project = client.get_project(project_id.clone()).unwrap();
    // assert_eq!(project.project_id, project_id);
    // assert_eq!(project.creator, creator);
    // assert_eq!(project.metadata_uri, metadata_uri);
    
}

#[test]
#[should_panic]
fn test_create_project_already_exists() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);    
}

#[test]
#[should_panic]
fn test_create_project_invalid_funding_target() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);
    
    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 0;
    let milestone_count = 5;
    
    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
}

#[test]
#[should_panic]
fn test_create_project_invalid_milestone_count() {
    let env = Env::default();
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let project_id = String::from_str(&env, "test_project");
    let metadata_uri = String::from_str(&env, "https://example.com/metadata");
    let funding_target = 1000;
    let milestone_count = 101;

    client.create_project(&project_id, &creator, &metadata_uri, &funding_target, &milestone_count);
}

#[test]
fn test_update_project_metadata() {
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
    let new_metadata_uri = String::from_str(&env, "https://example.com/new_metadata");
    client.update_project_metadata(&project_id, &creator, &new_metadata_uri);
    let project = client.get_project(&project_id);
    assert_eq!(project.metadata_uri, new_metadata_uri);
}

#[test]
#[should_panic]
fn test_update_project_metadata_unauthorized() {
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
    let unauthorized_address = Address::generate(&env);
    client.update_project_metadata(&project_id, &unauthorized_address, &metadata_uri);
}

#[test]
fn test_update_project_milestone_count() {
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
    let new_milestone_count = 10;
    client.update_project_milestone_count(&project_id, &creator, &new_milestone_count);
    let project = client.get_project(&project_id);
    assert_eq!(project.milestone_count, new_milestone_count);
}

#[test]
#[should_panic]
fn test_update_project_milestone_count_unauthorized() {
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
    let unauthorized_address = Address::generate(&env);
    client.update_project_milestone_count(&project_id, &unauthorized_address, &milestone_count);
}

#[test]
fn test_modify_milestone() {
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
    let new_milestone_count = 10;
    client.modify_milestone(&project_id, &creator, &new_milestone_count);
    let project = client.get_project(&project_id);
    assert_eq!(project.milestone_count, new_milestone_count);
}

#[test]
#[should_panic]
fn test_modify_milestone_unauthorized() {
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
    let unauthorized_address = Address::generate(&env);
    client.modify_milestone(&project_id, &unauthorized_address, &milestone_count);
}

