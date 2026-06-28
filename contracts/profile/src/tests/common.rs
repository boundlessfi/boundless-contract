// boundless-profile: shared test setup.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{ProfileContract, ProfileContractClient};

pub struct TestCtx<'a> {
    #[allow(dead_code)]
    pub env: Env,
    pub admin: Address,
    pub client: ProfileContractClient<'a>,
}

pub fn setup<'a>() -> TestCtx<'a> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);

    let contract_id = env.register(ProfileContract, (admin.clone(),));
    let client = ProfileContractClient::new(&env, &contract_id);

    TestCtx { env, admin, client }
}
