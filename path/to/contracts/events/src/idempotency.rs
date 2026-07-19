// Import necessary dependencies
use std::ops::Add;
use std::result::Result;

// Define the Error enum
#[derive(Debug)]
pub enum Error {
    EventIdOverflow,
}

// Modify the next_event_id function to use checked_add and handle overflows
pub fn next_event_id(current: u64, increment: u64) -> Result<u64, Error> {
    current.checked_add(increment).ok_or(Error::EventIdOverflow)
}

// Update the stored_next_event_id function to use checked_add and handle overflows
pub fn stored_next_event_id(current: u64, increment: u64) -> Result<u64, Error> {
    current.checked_add(increment).ok_or(Error::EventIdOverflow)
}