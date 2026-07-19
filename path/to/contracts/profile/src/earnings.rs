// Import necessary dependencies
use std::ops::Add;
use std::result::Result;

// Define the Error enum
#[derive(Debug)]
pub enum Error {
    EarningsOverflow,
}

// Modify the register function to use checked_add and handle overflows
pub fn register(current: i128, amount: i128) -> Result<i128, Error> {
    current.checked_add(amount).ok_or(Error::EarningsOverflow)
}