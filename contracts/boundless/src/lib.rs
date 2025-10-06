#![no_std]

use soroban_sdk::contract;

mod datatypes;
mod interface;
mod logic;
mod helper;

pub use logic::*;

#[contract]
pub struct BoundlessContract;

mod tests;
