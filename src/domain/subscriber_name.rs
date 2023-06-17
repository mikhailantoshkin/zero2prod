use std::fmt::Display;

use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

const FORBIDEN_CHARS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

pub enum NameValidationError {
    TooLong,
    Empy,
    FrobidenCharecter,
}

impl Display for NameValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid name: {}",
            match self {
                NameValidationError::Empy => "name is empty",
                NameValidationError::FrobidenCharecter => "name contains forbidden character",
                NameValidationError::TooLong => "name is too long",
            }
        )
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, NameValidationError> {
        if s.trim().is_empty() {
            return Err(NameValidationError::Empy);
        }
        if s.graphemes(true).count() > 256 {
            return Err(NameValidationError::TooLong);
        }
        if s.chars().any(|g| FORBIDEN_CHARS.contains(&g)) {
            return Err(NameValidationError::FrobidenCharecter);
        }
        Ok(Self(s))
    }

    pub fn inner(self) -> String {
        self.0
    }
}
impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubscriberName {
    type Error = NameValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        SubscriberName::parse(value)
    }
}

impl Display for SubscriberName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::subscriber_name::{NameValidationError, SubscriberName, FORBIDEN_CHARS};

    #[test]
    fn a_256_grapheme_name_is_valid() {
        let name = "a".repeat(256);
        assert!(SubscriberName::parse(name).is_ok())
    }
    #[test]
    fn long_name_is_invalid() {
        let name = "a".repeat(257);
        assert!(SubscriberName::parse(name).is_err_and(|x| match x {
            NameValidationError::TooLong => true,
            _ => false,
        }))
    }
    #[test]
    fn witespace_name_is_invalid() {
        let name = " ".to_string();
        assert!(SubscriberName::parse(name).is_err_and(|x| match x {
            NameValidationError::Empy => true,
            _ => false,
        }))
    }
    #[test]
    fn empyt_name_is_invalid() {
        let name = "".to_string();
        assert!(SubscriberName::parse(name).is_err_and(|x| match x {
            NameValidationError::Empy => true,
            _ => false,
        }))
    }
    #[test]
    fn name_with_invallid_chars_is_rejected() {
        for name in FORBIDEN_CHARS {
            assert!(
                SubscriberName::parse(name.to_string()).is_err_and(|x| match x {
                    NameValidationError::FrobidenCharecter => true,
                    _ => false,
                })
            )
        }
    }
    #[test]
    fn valid_name_is_parsed() {
        let name = "Poppy Bowling".to_string();
        assert!(SubscriberName::parse(name).is_ok())
    }
}
