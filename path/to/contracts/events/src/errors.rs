// Import necessary dependencies
use std::error::Error;

// Define the Error enum
#[derive(Debug)]
pub enum Error {
    EventIdOverflow,
}

// Implement the Error trait for the Error enum
impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::EventIdOverflow => "Event ID overflow",
        }
    }
}