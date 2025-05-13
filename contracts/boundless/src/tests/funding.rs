#![cfg(test)]

use crate::{
    datatypes::{
        BackerContribution, BoundlessError, DataKey, Project, ProjectFundedEvent, ProjectStatus,
        FUNDING_PERIOD_LEDGERS,
    },
    BoundlessContract, BoundlessContractClient,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String, Vec,
};

fn setup_test_env<'a>() -> (
    Env,
    Address,
    StellarAssetClient<'a>,
    TokenClient<'a>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());
    let client = BoundlessContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let stellar_asset = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = stellar_asset.address();
    let token_admin = StellarAssetClient::new(&env, &token_id);
    let token_client = TokenClient::new(&env, &token_id);

    let user = Address::generate(&env);
    token_admin.mint(&user, &1000000);

    (env, contract_id, token_admin, token_client, admin, user)
}

fn setup_create_project_args(env: &Env) -> (String, String, u64, u32) {
    // Define create_project function args
    let project_id = String::from_str(&env, "test-project");
    let metadata_uri = String::from_str(&env, "ipfs://test");
    let funding_target = 500000_u64;
    let milestone_count = 5_u32;

    (project_id, metadata_uri, funding_target, milestone_count)
}

#[test]
fn test_successful_funding() {
    // Create and initialize the test environment and contract clients
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    // Define create_project function args snd create project
    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    // Set project status to funding
    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Whitelist token contract
    client.whitelist_token_contract(&admin, &project_id, &token.address);

    let user_balance_before: i128 = token.balance(&user);
    let contract_balance_before: i128 = token.balance(&contract_id);

    // Fund project with a valid amount
    let funding_amount = 200000_i128;
    let result = client.fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(result, ());

    // Check if the funding amounts are correct
    let (total_funded, target) = client.get_project_funding(&project_id);
    assert_eq!(total_funded, funding_amount as u64);
    assert_eq!(target, funding_target);

    // Check if the project balance is updated correctly
    let user_balance_after: i128 = token.balance(&user);
    assert_eq!(
        user_balance_after,
        user_balance_before - funding_amount as i128
    );
    let contract_balance_after: i128 = token.balance(&contract_id);
    assert_eq!(
        contract_balance_after,
        contract_balance_before + funding_amount as i128
    );

 let events = env.events().all();
    // log!(&env, "Captured events: {:?}", events);
    // assert_eq!(events.len(), 1, "Expected one initialization event");
    // assert_eq!(
    //     events,
    //     vec![
    //         &env,
    //         (
    //             governance_id.clone(),
    //             (symbol_short!("govern"), symbol_short!("init")).into_val(&env),
    //             (
    //                 admin.clone(),
    //                 token.clone(),
    //                 referral.clone(),
    //                 auction.clone()
    //             )
    //                 .into_val(&env)
    //         ),
    //     ],
    //     "Initialization event mismatch"
    // );
}

#[test]
fn test_funding_insufficient_funds() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);
    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    // Attempt to fund project with insufficient funds
    let funding_amount = 0_i128;
    let result = client.try_fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InsufficientFunds)),
        "Expected InsufficientFunds error"
    );
}

#[test]
fn test_funding_invalid_project_status() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    // Attempt to fund project while in Voting stage
    let funding_amount = 200000_i128;
    let result = client.try_fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InvalidOperation)),
        "Expected InvalidOperation error"
    );
}

#[test]
fn test_funding_project_closed() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);
    client.close_project(&project_id, &user);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Attempt to fund project when project is closed
    let funding_amount = 200000_i128;
    let result = client.try_fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::ProjectClosed)),
        "Expected ProjectClosed error"
    );
}

#[test]
fn test_funding_past_deadline() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Fast forward the ledger time to simulate funding deadline reached
    env.ledger().with_mut(|li| {
        li.timestamp += (FUNDING_PERIOD_LEDGERS + 1) as u64;
    });

    // Attempt to fund project when funding deadline is reached
    let funding_amount = 200000_i128;
    let result = client.try_fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::FundingPeriodEnded)),
        "Expected FundingPeriodEnded error"
    );
}

#[test]
fn test_funding_invalid_token() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Attempt to fund project without whitelisting token contract
    let funding_amount = 200000_i128;
    let result = client.try_fund_project(&project_id, &funding_amount, &user, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InvalidTokenContract)),
        "Expected InvalidTokenContract error"
    );
}

#[test]
fn test_funding_same_backer_multiple_contributions() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount1 = 200000_i128;
    let funding_amount2 = 100000_i128;
    client.fund_project(&project_id, &funding_amount1, &user, &token.address);
    client.fund_project(&project_id, &funding_amount2, &user, &token.address);

    let funding_total = (funding_amount1 + funding_amount2) as u64;
    let (total_funded, _) = client.get_project_funding(&project_id);
    assert_eq!(total_funded, funding_total);

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert_eq!(project.status, ProjectStatus::Funding); // Funding target not hit
        assert_eq!(project.backers.len(), 1);
        assert_eq!(project.backers.get(0).unwrap().0, user);
        assert_eq!(project.backers.get(0).unwrap().1, funding_total);

        let backers: Vec<BackerContribution> = env
            .storage()
            .persistent()
            .get(&DataKey::Backers(project_id.clone()))
            .unwrap();
        assert_eq!(backers.len(), 2);
        assert_eq!(backers.get(0).unwrap().backer, user);
        assert_eq!(backers.get(1).unwrap().backer, user);
        assert_eq!(backers.get(0).unwrap().amount, funding_amount1 as u64);
        assert_eq!(backers.get(1).unwrap().amount, funding_amount2 as u64);
    });
}

#[test]
fn test_fund_same_backer_multiple_tokens() {
    // Setup test environment
    let (env, contract_id, _, token_client1, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    // Create a second token contract and mint tokens
    let stellar_asset = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id2 = stellar_asset.address();
    let token_admin2 = StellarAssetClient::new(&env, &token_id2);
    let token_client2 = TokenClient::new(&env, &token_id2);

    token_admin2.mint(&user, &500000);

    // Create a project
    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    // Whitelist both token contracts
    client.whitelist_token_contract(&admin, &project_id, &token_client1.address);
    client.whitelist_token_contract(&admin, &project_id, &token_client2.address);

    // Set project status to funding
    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Get token balances before funding
    let user_token1_balance_before_funding: i128 = token_client1.balance(&user);
    let user_token2_balance_before_funding: i128 = token_client2.balance(&user);
    let contract_token1_balance_before_funding: i128 = token_client1.balance(&contract_id);
    let contract_token2_balance_before_funding: i128 = token_client2.balance(&contract_id);

    // Fund project with multiple backers using different tokens
    let funding_amount = 250000_i128;
    let funding_total = funding_amount * 2;

    client.fund_project(&project_id, &funding_amount, &user, &token_client1.address);
    client.fund_project(&project_id, &funding_amount, &user, &token_client2.address);

    // Compare token balances after funding
    let user_token1_balance_after_funding: i128 = token_client1.balance(&user);
    let user_token2_balance_after_funding: i128 = token_client2.balance(&user);
    let contract_token1_balance_after_funding: i128 = token_client1.balance(&contract_id);
    let contract_token2_balance_after_funding: i128 = token_client2.balance(&contract_id);
    assert_eq!(
        user_token1_balance_after_funding,
        user_token1_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        user_token2_balance_after_funding,
        user_token2_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        contract_token1_balance_after_funding,
        contract_token1_balance_before_funding + funding_amount as i128
    );
    assert_eq!(
        contract_token2_balance_after_funding,
        contract_token2_balance_before_funding + funding_amount as i128
    );
}

#[test]
fn test_funding_multiple_backers() {
    let (env, contract_id, token_admin, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);
    let backer = Address::generate(&env);
    token_admin.mint(&backer, &1000000);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount1 = 200000_i128;
    let funding_amount2 = 250000_i128;
    client.fund_project(&project_id, &funding_amount1, &user, &token.address);
    client.fund_project(&project_id, &funding_amount2, &backer, &token.address);

    let (total_funded, _) = client.get_project_funding(&project_id);
    assert_eq!(total_funded, (funding_amount1 + funding_amount2) as u64);

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert_eq!(project.backers.len(), 2);
        assert_eq!(project.backers.get(0).unwrap().0, user);
        assert_eq!(project.backers.get(0).unwrap().1, funding_amount1 as u64);
        assert_eq!(project.backers.get(1).unwrap().0, backer);
        assert_eq!(project.backers.get(1).unwrap().1, funding_amount2 as u64);

        let backers: Vec<BackerContribution> = env
            .storage()
            .persistent()
            .get(&DataKey::Backers(project_id.clone()))
            .unwrap();
        assert_eq!(backers.len(), 2);
        assert_eq!(backers.get(0).unwrap().backer, user);
        assert_eq!(backers.get(1).unwrap().backer, backer);
        assert_eq!(backers.get(0).unwrap().amount, funding_amount1 as u64);
        assert_eq!(backers.get(1).unwrap().amount, funding_amount2 as u64);
    });

    let user_contribution = client.get_backer_contribution(&project_id, &user);
    let backer_contribution = client.get_backer_contribution(&project_id, &backer);
    assert_eq!(user_contribution, funding_amount1 as u64);
    assert_eq!(backer_contribution, funding_amount2 as u64);
}

#[test]
fn test_funding_multiple_contributions_hit_target() {
    let (env, contract_id, token_admin, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);
    let backer = Address::generate(&env);
    token_admin.mint(&backer, &1000000);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let user_balance_before: i128 = token.balance(&user);
    let backer_balance_before: i128 = token.balance(&backer);
    let contract_balance_before: i128 = token.balance(&contract_id);

    let funding_amount1 = 250000_i128;
    let funding_amount2 = 250000_i128;
    client.fund_project(&project_id, &funding_amount1, &user, &token.address);
    client.fund_project(&project_id, &funding_amount2, &backer, &token.address);

    let funding_total = (funding_amount1 + funding_amount2) as u64;
    let (total_funded, target) = client.get_project_funding(&project_id);
    assert_eq!(total_funded, funding_total);
    assert_eq!(total_funded, target);

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert_eq!(project.status, ProjectStatus::Funded); // Funding target hit
    });

    let user_balance_after: i128 = token.balance(&user);
    let backer_balance_after: i128 = token.balance(&backer);
    let contract_balance_after: i128 = token.balance(&contract_id);
    assert_eq!(
        user_balance_after,
        user_balance_before - funding_amount1 as i128
    );
    assert_eq!(
        backer_balance_after,
        backer_balance_before - funding_amount2 as i128
    );
    assert_eq!(
        contract_balance_after,
        contract_balance_before + funding_total as i128
    );
}

#[test]
fn test_refund_scenario() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount = 200000_i128;
    client.fund_project(&project_id, &funding_amount, &user, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Voting;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    client.close_project(&project_id, &user);

    client.refund(&project_id, &token.address);

    let project = client.get_project(&project_id);
    assert!(project.refund_processed);
}

#[test]
fn test_refund_invalid_project_id() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let project_id = String::from_str(&env, "nonexistent-project");

    // Attempt to refund a non-existent project
    let result = client.try_refund(&project_id, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::NotFound)),
        "Expected NotFound error"
    );
}

#[test]
fn test_refund_invalid_project_status() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount = 200000_i128;
    client.fund_project(&project_id, &funding_amount, &user, &token.address);

    // Attempt to refund without closing project or failed project status
    let result = client.try_refund(&project_id, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InvalidOperation)),
        "Expected InvalidOperation error"
    );
}

#[test]
fn test_refund_non_whitelisted_token() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount = 200000_i128;
    client.fund_project(&project_id, &funding_amount, &user, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Failed;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Attempt to refund non-whitelisted token contract
    let invalid_token = Address::generate(&env);
    let result = client.try_refund(&project_id, &invalid_token);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InvalidTokenContract)),
        "Expected InvalidTokenContract error"
    );
}

#[test]
fn test_refund_already_processed() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount = 200000_i128;
    client.fund_project(&project_id, &funding_amount, &user, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Voting;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    client.close_project(&project_id, &user);
    client.refund(&project_id, &token.address);

    // Attempt to refund an already refunded project
    let result = client.try_refund(&project_id, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::RefundAlreadyProcessed)),
        "Expected RefundAlreadyProcessed error"
    );
}

#[test]
fn test_refund_no_backer_contributions() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Voting;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    client.close_project(&project_id, &user);

    // Attempt to refund project with zero contributions
    let result = client.try_refund(&project_id, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::NoBackerContributions)),
        "Expected NoBackerContributions error"
    );

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert_eq!(project.backers.len(), 0);
    });
}

#[test]
fn test_refund_insufficient_funds() {
    let (env, contract_id, _, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    let funding_amount = 200000_i128;
    client.fund_project(&project_id, &funding_amount, &user, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Failed;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    token.burn(&contract_id, &funding_amount);

    // Attempt to refund failed project that has insufficient funds after burning tokens
    let result = client.try_refund(&project_id, &token.address);
    assert_eq!(
        result,
        Err(Ok(BoundlessError::InsufficientFunds)),
        "Expected InsufficientFunds error"
    );
}

#[test]
fn test_refund_multiple_backers() {
    let (env, contract_id, token_admin, token, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);
    let backer1 = Address::generate(&env);
    let backer2 = Address::generate(&env);
    token_admin.mint(&backer1, &500000);
    token_admin.mint(&backer2, &500000);

    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    client.whitelist_token_contract(&admin, &project_id, &token.address);

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Get token balances before funding
    let backer1_balance_before_funding: i128 = token.balance(&backer1);
    let backer2_balance_before_funding: i128 = token.balance(&backer2);
    let contract_balance_before_funding: i128 = token.balance(&contract_id);

    let funding_amount1 = 200000_i128;
    let funding_amount2 = 300000_i128;
    let funding_total = funding_amount1 + funding_amount2;

    // Fund project with multiple backers
    client.fund_project(&project_id, &funding_amount1, &backer1, &token.address);
    client.fund_project(&project_id, &funding_amount2, &backer2, &token.address);

    // Compare token balances after funding
    let backer1_balance_after_funding: i128 = token.balance(&backer1);
    let backer2_balance_after_funding: i128 = token.balance(&backer2);
    let contract_balance_after_funding: i128 = token.balance(&contract_id);
    assert_eq!(
        backer1_balance_after_funding,
        backer1_balance_before_funding - funding_amount1 as i128
    );
    assert_eq!(
        backer2_balance_after_funding,
        backer2_balance_before_funding - funding_amount2 as i128
    );
    assert_eq!(
        contract_balance_after_funding,
        contract_balance_before_funding + funding_total
    );

    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Failed;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Attempt to refund multiple backers
    client.refund(&project_id, &token.address);

    // Compare token balances after refund
    let backer1_balance_after_refund: i128 = token.balance(&backer1);
    let backer2_balance_after_refund: i128 = token.balance(&backer2);
    let contract_balance_after_refund: i128 = token.balance(&contract_id);
    assert_eq!(
        backer1_balance_after_refund,
        backer1_balance_after_funding + funding_amount1 as i128
    );
    assert_eq!(
        backer2_balance_after_refund,
        backer2_balance_after_funding + funding_amount2 as i128
    );
    assert_eq!(
        contract_balance_after_refund,
        contract_balance_after_funding - funding_total
    );
    assert_eq!(
        contract_balance_after_refund,
        contract_balance_before_funding
    );

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert!(project.refund_processed);
    });
}

#[test]
fn test_refund_multiple_tokens() {
    // Setup test environment
    let (env, contract_id, token_admin1, token_client1, admin, user) = setup_test_env();
    let client = BoundlessContractClient::new(&env, &contract_id);
    let backer1 = Address::generate(&env);
    let backer2 = Address::generate(&env);

    // Mint tokens to their respectives balances
    token_admin1.mint(&backer1, &100000);
    token_admin1.mint(&backer2, &100000);

    // Create a second token contract and mint tokens
    let stellar_asset = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id2 = stellar_asset.address();
    let token_admin2 = StellarAssetClient::new(&env, &token_id2);
    let token_client2 = TokenClient::new(&env, &token_id2);

    token_admin2.mint(&backer1, &100000);
    token_admin2.mint(&backer2, &100000);

    // Create a project
    let (project_id, metadata_uri, funding_target, milestone_count) =
        setup_create_project_args(&env);

    client.create_project(
        &project_id,
        &user,
        &metadata_uri,
        &funding_target,
        &milestone_count,
    );

    // Whitelist both token contracts
    client.whitelist_token_contract(&admin, &project_id, &token_client1.address);
    client.whitelist_token_contract(&admin, &project_id, &token_client2.address);

    // Set project status to funding
    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Funding;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Get token balances before funding
    let backer1_token1_balance_before_funding: i128 = token_client1.balance(&backer1);
    let backer2_token1_balance_before_funding: i128 = token_client1.balance(&backer2);
    let contract_token1_balance_before_funding: i128 = token_client1.balance(&contract_id);

    let backer1_token2_balance_before_funding: i128 = token_client2.balance(&backer1);
    let backer2_token2_balance_before_funding: i128 = token_client2.balance(&backer2);
    let contract_token2_balance_before_funding: i128 = token_client2.balance(&contract_id);

    // Fund project with multiple backers using different tokens
    let funding_amount = 100000_i128;
    let funding_total = funding_amount * 2;

    client.fund_project(
        &project_id,
        &funding_amount,
        &backer1,
        &token_client1.address,
    );
    client.fund_project(
        &project_id,
        &funding_amount,
        &backer2,
        &token_client1.address,
    );
    client.fund_project(
        &project_id,
        &funding_amount,
        &backer1,
        &token_client2.address,
    );
    client.fund_project(
        &project_id,
        &funding_amount,
        &backer2,
        &token_client2.address,
    );

    // Compare token balances after funding
    let backer1_token1_balance_after_funding: i128 = token_client1.balance(&backer1);
    let backer2_token1_balance_after_funding: i128 = token_client1.balance(&backer2);
    let contract_token1_balance_after_funding: i128 = token_client1.balance(&contract_id);
    assert_eq!(
        backer1_token1_balance_after_funding,
        backer1_token1_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        backer2_token1_balance_after_funding,
        backer2_token1_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        contract_token1_balance_after_funding,
        contract_token1_balance_before_funding + funding_total as i128
    );

    let backer1_token2_balance_after_funding: i128 = token_client2.balance(&backer1);
    let backer2_token2_balance_after_funding: i128 = token_client2.balance(&backer2);
    let contract_token2_balance_after_funding: i128 = token_client2.balance(&contract_id);
    assert_eq!(
        backer1_token2_balance_after_funding,
        backer1_token2_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        backer2_token2_balance_after_funding,
        backer2_token2_balance_before_funding - funding_amount as i128
    );
    assert_eq!(
        contract_token2_balance_after_funding,
        contract_token2_balance_before_funding + funding_total as i128
    );

    // Set project status to failed to enable refunds
    env.as_contract(&contract_id, || {
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        project.status = ProjectStatus::Failed;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);
    });

    // Attempt to refund multiple backers with multiple tokens
    client.refund(&project_id, &token_client1.address);
    client.refund(&project_id, &token_client2.address);

    // Compare token balances after refund
    let backer1_token1_balance_after_refund: i128 = token_client1.balance(&backer1);
    let backer2_token1_balance_after_refund: i128 = token_client1.balance(&backer2);
    let contract_token1_balance_after_refund: i128 = token_client1.balance(&contract_id);
    assert_eq!(
        backer1_token1_balance_after_refund,
        backer1_token1_balance_after_funding + funding_amount as i128
    );
    assert_eq!(
        backer2_token1_balance_after_refund,
        backer2_token1_balance_after_funding + funding_amount as i128
    );
    assert_eq!(
        contract_token1_balance_after_refund,
        contract_token1_balance_after_funding - funding_total as i128
    );
    assert_eq!(
        contract_token1_balance_after_refund,
        contract_token1_balance_before_funding
    );

    let backer1_token2_balance_after_refund: i128 = token_client2.balance(&backer1);
    let backer2_token2_balance_after_refund: i128 = token_client2.balance(&backer2);
    let contract_token2_balance_after_refund: i128 = token_client2.balance(&contract_id);
    assert_eq!(
        backer1_token2_balance_after_refund,
        backer1_token2_balance_after_funding + funding_amount as i128
    );
    assert_eq!(
        backer2_token2_balance_after_refund,
        backer2_token2_balance_after_funding + funding_amount as i128
    );
    assert_eq!(
        contract_token2_balance_after_refund,
        contract_token2_balance_after_funding - funding_total as i128
    );
    assert_eq!(
        contract_token2_balance_after_refund,
        contract_token2_balance_before_funding
    );

    env.as_contract(&contract_id, || {
        let project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id.clone()))
            .unwrap();
        assert!(project.refund_processed);

        // Check whitelisted tokens
        let whitelisted_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::WhitelistedTokens(project_id.clone()))
            .unwrap();
        assert_eq!(whitelisted_tokens.len(), 2);
        assert_eq!(whitelisted_tokens.get(0).unwrap(), token_client1.address);
        assert_eq!(whitelisted_tokens.get(1).unwrap(), token_client2.address);

        // Check refunded tokens
        let refunded_tokens: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::RefundedTokens(project_id.clone()))
            .unwrap();

        assert_eq!(refunded_tokens.len(), 2);
        assert_eq!(refunded_tokens.get(0).unwrap(), token_client1.address);
        assert_eq!(refunded_tokens.get(1).unwrap(), token_client2.address);
    });
}
