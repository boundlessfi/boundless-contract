// boundless-events: shared test setup.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    Address, BytesN, Env,
};

use crate::event_ops::MAX_REFUNDS_PER_BATCH;
use crate::types::EventStatus;
use crate::{EventsContract, EventsContractClient};

/// Drive a paged cancel end-to-end for the given event. Used by tests so
/// each call site does not have to thread start_cancel / process /
/// finalize manually. The OwnerOnly branch settles inside start_cancel,
/// so an event with no partner contributions skips the batch + finalize
/// steps.
#[allow(dead_code)]
pub fn drive_cancel<'a>(env: &Env, client: &EventsContractClient<'a>, id: u64) {
    let op_start = BytesN::random(env);
    client.start_cancel(&id, &op_start);

    let after_start = client.get_event(&id);
    if matches!(after_start.status, EventStatus::Cancelled) {
        return;
    }

    loop {
        let op = BytesN::random(env);
        let remaining = client.process_cancel_batch(&id, &MAX_REFUNDS_PER_BATCH, &op);
        if remaining == 0 {
            break;
        }
    }

    let op_final = BytesN::random(env);
    client.finalize_cancel(&id, &op_final);
}

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
