#![cfg(test)]

use crate::{
    datatypes::{
        BoundlessError, MilestoneStatus, Project, ProjectStatus, FUNDING_PERIOD_LEDGERS,
        VOTING_PERIOD_LEDGERS,
    },
    BoundlessContract, BoundlessContractClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

// #[test]
// fn test_release_milestone() {
//     let env = Env::default();
//     let contract_id = env.register(BoundlessContract, ());

//     env.mock_all_auths();
//     let client = BoundlessContractClient::new(&env, &contract_id);

//     let admin = Address::generate(&env);
//     client.initialize(&admin);
//     let project_id = String::from_str(&env, "test_project");
//     let metadata_uri = String::from_str(&env, "https://example.com/metadata");
//     let funding_target = 1000;
//     let milestone_count = 5;

//     client.create_project(
//         &project_id,
//         &admin,
//         &metadata_uri,
//         &funding_target,
//         &milestone_count,
//     );
//     let voter = Address::generate(&env);
//     client.vote_project(&project_id, &voter, &1);
//     let milestone_number = 1;
//     // let milestone_amount = 100;
//     client.release_milestone(&project_id, &milestone_number, &admin);
//     let milestone_status = client.get_milestone_status(&project_id, &milestone_number);
//     assert_eq!(milestone_status, MilestoneStatus::Released);
// }
