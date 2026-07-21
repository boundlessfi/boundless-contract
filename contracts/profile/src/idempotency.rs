use soroban_sdk::{Address, BytesN, Env};

use crate::errors::Error;
use crate::storage;

pub fn require_unseen(env: &Env, domain: &Address, op_id: &BytesN<32>) -> Result<(), Error> {
    if storage::is_op_seen(env, domain, op_id) {
        return Err(Error::OpAlreadySeen);
    }
    Ok(())
}

pub fn mark_seen(env: &Env, domain: &Address, op_id: &BytesN<32>) {
    storage::mark_op_seen(env, domain, op_id);
}

/// Domain for ops authorized by the configured events contract.
pub fn events_domain(env: &Env) -> Result<Address, Error> {
    storage::get_events_contract(env).ok_or(Error::EventsContractNotConfigured)
}
