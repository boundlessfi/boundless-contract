#![no_std]

use soroban_sdk::contract;

pub use logic::*;

mod logic;
mod datatypes;
mod interface;

#[contract]
pub struct BoundlessContract;

// mod tests;
