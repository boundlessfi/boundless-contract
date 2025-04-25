#![no_std]

use soroban_sdk::contract;

pub use logic::*;

mod datatypes;
mod interface;
mod logic;

#[contract]
pub struct BoundlessContract;

// mod tests;
