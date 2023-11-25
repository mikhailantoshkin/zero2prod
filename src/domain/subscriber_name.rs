use std::fmt::Display;

use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

const FORBIDDEN_CHARS: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

pub enum NameValidationError {
    TooLong,
    Empty,
    ForbiddenCharacter,
}

impl Display for NameValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid name: {}",
            match self {
                NameValidationError::Empty => "name is empty",
                NameValidationError::ForbiddenCharacter => "name contains forbidden character",
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
            return Err(NameValidationError::Empty);
        }
        if s.graphemes(true).count() > 256 {
            return Err(NameValidationError::TooLong);
        }
        if s.chars().any(|g| FORBIDDEN_CHARS.contains(&g)) {
            return Err(NameValidationError::ForbiddenCharacter);
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
    use crate::domain::subscriber_name::{NameValidationError, SubscriberName, FORBIDDEN_CHARS};

    #[test]
    fn a_256_grapheme_name_is_valid() {
        let name = "a".repeat(256);
        assert!(SubscriberName::parse(name).is_ok())
    }
    #[test]
    fn long_name_is_invalid() {
        let name = "a".repeat(257);
        assert!(
            SubscriberName::parse(name).is_err_and(|x| matches!(x, NameValidationError::TooLong))
        )
    }
    #[test]
    fn whitespace_name_is_invalid() {
        let name = " ".to_string();
        assert!(SubscriberName::parse(name).is_err_and(|x| matches!(x, NameValidationError::Empty)))
    }
    #[test]
    fn empty_name_is_invalid() {
        let name = "".to_string();
        assert!(SubscriberName::parse(name).is_err_and(|x| matches!(x, NameValidationError::Empty)))
    }
    #[test]
    fn name_with_invalid_chars_is_rejected() {
        for name in FORBIDDEN_CHARS {
            assert!(SubscriberName::parse(name.to_string())
                .is_err_and(|x| matches!(x, NameValidationError::ForbiddenCharacter)))
        }
    }
    #[test]
    fn valid_name_is_parsed() {
        let name = "Poppy Bowling".to_string();
        assert!(SubscriberName::parse(name).is_ok())
    }
}
