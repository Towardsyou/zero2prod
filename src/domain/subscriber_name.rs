use std::str::FromStr;

use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl FromStr for SubscriberName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let is_empty_or_whitespace = s.trim().is_empty();
        // A grapheme is defined by the Unicode standard as a "user-perceived"
        // character: `å` is a single grapheme, but it is composed of two characters
        // (`a` and `̊`).
        //
        // `graphemes` returns an iterator over the graphemes in the input `s`.
        // `true` specifies that we want to use the extended grapheme definition set,
        // the recommended one.
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err("Invalid subscriber name".into())
        } else {
            Ok(SubscriberName(s.to_string()))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_good_names_can_pass() {
        let name = "Ursula Le Guin".to_string();
        claim::assert_ok!(SubscriberName::from_str(&name));
    }

    #[test]
    fn test_name_length_check() {
        let name = "a".repeat(256);
        claim::assert_ok!(SubscriberName::from_str(&name));
        let name = "a".repeat(257);
        claim::assert_err!(SubscriberName::from_str(&name));
    }

    #[test]
    fn empty_or_whitespace_only_names_are_rejected() {
        let name = "  ";
        claim::assert_err!(SubscriberName::from_str(&name));
        let name = "";
        claim::assert_err!(SubscriberName::from_str(&name));
    }

    #[test]
    fn forbidden_characters_are_rejected() {
        for c in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = format!("a{c}a");
            claim::assert_err!(SubscriberName::from_str(&name));
        }
    }
}
