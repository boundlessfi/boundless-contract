// boundless-events: token whitelist management.
//
// Spec: boundless-platform-contract-prd.md Section 8.
//
// M2 (2026-06 audit): Trustline verification for the fee account is the
// admin's off-chain responsibility (runbook in boundless-infra). On-chain
// enforcement isn't reliable in Soroban today: Stellar Asset Contract's
// `balance(addr)` returns 0 for "no trustline" and 0 for "trustline at
// zero balance" indistinguishably, and a 0-amount `transfer` probe behaves
// inconsistently across SAC variants. Until a non-standard token interface
// extension lands, the chosen policy is:
//
//   1. Admin verifies trustline existence off-chain BEFORE calling
//      register (this module) or set_fee_account (admin module).
//   2. On register/deregister we emit TokenRegistered/TokenDeregistered.
//      Off-chain monitors check trustline state and surface alarms if a
//      newly-registered token doesn't have a fee-account trustline.
//   3. If trustline is missing, the first add_funds / deposit on the token
//      will revert inside SAC.transfer. The admin then rotates either the
//      fee account (via set_fee_account) or deregisters the token.
//
// See docs/audit-2026-06-stellar-skill.md M2.
#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::admin;
use crate::errors::Error;
use crate::events as evt;
use crate::storage;

pub fn register(env: &Env, token: Address) -> Result<(), Error> {
    admin::require_admin(env)?;

    storage::set_token_supported(env, &token, true);
    storage::append_supported_token(env, &token);
    storage::touch_instance(env);
    evt::TokenRegistered {
        token: token.clone(),
    }
    .publish(env);
    Ok(())
}

pub fn deregister(env: &Env, token: Address) -> Result<(), Error> {
    admin::require_admin(env)?;
    storage::set_token_supported(env, &token, false);
    storage::remove_supported_token(env, &token);
    storage::touch_instance(env);
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
