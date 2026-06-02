// boundless-profile: idempotency helpers.

use soroban_sdk::{BytesN, Env};

use crate::errors::Error;
use crate::storage;

pub fn require_unseen(env: &Env, op_id: &BytesN<32>) -> Result<(), Error> {
    if storage::is_op_seen(env, op_id) {
        return Err(Error::OpAlreadySeen);
    }
    Ok(())
}

pub fn mark_seen(env: &Env, op_id: &BytesN<32>) {
    storage::mark_op_seen(env, op_id);
}
