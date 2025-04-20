#![no_std]
use crate::datatypes::DataKey;
use datatypes::BoundlessError;
use soroban_sdk::{contract, contractimpl, vec, Address, Env, String, Vec};

#[contract]
pub struct BoundlessContract;

#[contractimpl]
impl BoundlessContract {
    pub fn __construct(env: Env, admin: Address) -> Result<(), BoundlessError> {
        let mut admins = Vec::new(&env);
        admins.push_back(admin);
        env.storage().persistent().set(&DataKey::Admin, &admins);
        Ok(())
    }
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

pub use logic::{
    admin::*,
    project::*,
    voting::*,
    milestone::*,
};

mod datatypes;
mod interface;
mod logic;

mod test;
mod tests;