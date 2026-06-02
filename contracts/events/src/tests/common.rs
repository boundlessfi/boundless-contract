// boundless-events: shared test setup.

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{EventsContract, EventsContractClient};

pub struct TestCtx<'a> {
    #[allow(dead_code)]
    pub env: Env,
    pub admin: Address,
    pub fee_account: Address,
    pub profile_contract: Address,
    pub client: EventsContractClient<'a>,
}

pub fn setup<'a>(fee_bps: u32) -> TestCtx<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let profile_contract = Address::generate(&env);

    let contract_id = env.register(
        EventsContract,
        (
            admin.clone(),
            fee_account.clone(),
            fee_bps,
            profile_contract.clone(),
        ),
    );
    let client = EventsContractClient::new(&env, &contract_id);

    TestCtx {
        env,
        admin,
        fee_account,
        profile_contract,
        client,
    }
}
