use std::str::FromStr;

use validator::ValidateEmail;

pub struct SubscriberEmail(String);

impl FromStr for SubscriberEmail {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().validate_email() {
            Ok(Self(s.trim().to_string()))
        } else {
            Err("Invalid email address {s}".to_string())
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email_is_accepted() {
        let result = SubscriberEmail::from_str(" ursula@domain.com");
        assert!(result.is_ok());
    }

    #[test]
    fn empty_string_is_rejected(){
        let result = SubscriberEmail::from_str("");
        assert!(result.is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected(){
        let result = SubscriberEmail::from_str("example.com");
        assert!(result.is_err());
    }

    #[test]
    fn email_empty_left_of_at_symbol_is_rejected(){
        let result = SubscriberEmail::from_str("@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn email_empty_right_of_at_symbol_is_rejected(){
        let result = SubscriberEmail::from_str("example@");
        assert!(result.is_err());
    }
}