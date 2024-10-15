use std::str::FromStr;

use validator::ValidateEmail;

#[derive(Clone, Debug)]
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

    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn valid_email_is_accepted() {
        let result = SubscriberEmail::from_str(" ursula@domain.com");
        assert!(result.is_ok());

        let email: String = SafeEmail().fake();
        claim::assert_ok!(SubscriberEmail::from_str(&email));
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn randomly_generated_email_is_accepted(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::from_str(&valid_email.0).is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {
        let result = SubscriberEmail::from_str("");
        assert!(result.is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let result = SubscriberEmail::from_str("example.com");
        assert!(result.is_err());
    }

    #[test]
    fn email_empty_left_of_at_symbol_is_rejected() {
        let result = SubscriberEmail::from_str("@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn email_empty_right_of_at_symbol_is_rejected() {
        let result = SubscriberEmail::from_str("example@");
        assert!(result.is_err());
    }
}
