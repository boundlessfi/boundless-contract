// Import necessary dependencies
use super::*;

// Write unit tests to verify that overflows are properly handled and revert with a typed error
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_event_id_overflow() {
        let current = u64::MAX;
        let increment = 1;
        let result = next_event_id(current, increment);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::EventIdOverflow);
    }

    #[test]
    fn test_stored_next_event_id_overflow() {
        let current = u64::MAX;
        let increment = 1;
        let result = stored_next_event_id(current, increment);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::EventIdOverflow);
    }
}