// Import necessary dependencies
use super::*;

// Write unit tests to verify that overflows are properly handled and revert with a typed error
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_overflow() {
        let current = i128::MAX;
        let amount = 1;
        let result = register(current, amount);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::EarningsOverflow);
    }
}