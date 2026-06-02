// boundless-events: token whitelist management.
//
// Spec: boundless-platform-contract-prd.md Section 8.
//
// Trustline verification for the fee account is the admin's off-chain
// responsibility (runbook in boundless-infra). The contract layer cannot
// authorize a deeper-than-root call to the token, and a balance check would
// not actually pre-flight the trustline state. Admin verifies before calling.
//
// require_supported is wired by event-creation paths (stubbed). Allowed here
// to keep the warning floor clean while operation bodies land.
#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::storage;

pub fn register(env: &Env, token: Address) -> Result<(), Error> {
    admin::require_admin(env)?;

    storage::set_token_supported(env, &token, true);
    evt::TokenRegistered {
        token: token.clone(),
    }
    .publish(env);
    Ok(())
}

pub fn deregister(env: &Env, token: Address) -> Result<(), Error> {
    admin::require_admin(env)?;
    storage::set_token_supported(env, &token, false);
    evt::TokenDeregistered {
        token: token.clone(),
    }
    .publish(env);
    Ok(())
}

pub fn is_supported(env: &Env, token: &Address) -> bool {
    storage::is_token_supported(env, token)
}

pub fn require_supported(env: &Env, token: &Address) -> Result<(), Error> {
    if !storage::is_token_supported(env, token) {
        return Err(Error::TokenNotSupported);
    }
    Ok(())
}
