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
