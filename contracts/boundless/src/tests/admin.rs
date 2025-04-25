#![cfg(test)]

use crate::{datatypes::BoundlessError, BoundlessContract, BoundlessContractClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env,
};
extern crate std;
mod boundless {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/boundless.wasm"
    );
}
#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());

    let contract = BoundlessContractClient::new(&env, &contract_id);
    contract.initialize(&admin);

    assert_eq!(contract.get_admin(), admin);
    assert_eq!(contract.get_version(), 1);
}

#[test]
#[should_panic()]
fn test_initialize_already_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BoundlessContract, ());

    let contract = BoundlessContractClient::new(&env, &contract_id);
    contract.initialize(&admin);

    let err = contract.initialize(&admin);
}

// #[test]
// fn test_upgrade() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let admin = Address::generate(&env);
//     let contract_id = env.register(BoundlessContract, ());

//     let contract = BoundlessContractClient::new(&env, &contract_id);
//     contract.initialize(&admin);
//     // assert_eq!(1, contract.get_version());
//     let wasm = boundless::WASM;
//     let was_hash: BytesN<32> = env.deployer().upload_contract_wasm(boundless::WASM);
//     let upgrade_result = contract.upgrade(&was_hash);

//     assert_eq!(contract.get_version(), 2);
// }

// #[test]
// #[should_panic()]
// fn test_upgrade_not_admin() {
//     let env = Env::default();
//     // env.mock_auths();

//     let admin = Address::generate(&env);
//     let contract_id = env.register(BoundlessContract, ());

//     let contract = BoundlessContractClient::new(&env, &contract_id);
//     contract.initialize(&admin);

//     let not_admin = Address::generate(&env);
//     let was_hash: BytesN<32> = env.deployer().upload_contract_wasm(boundless::WASM);

//     let err = contract.upgrade(&was_hash);
//     // assert_eq!(err, BoundlessError::NotAuthorized);
// }
