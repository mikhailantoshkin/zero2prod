use std::fmt::Display;

use serde::{Deserialize, Serialize};
use validator::validate_email;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(try_from = "String")]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubscriberEmail {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        SubscriberEmail::parse(value)
    }
}

impl Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use fake::{faker::internet::en::SafeEmail, Fake};

    use crate::domain::SubscriberEmail;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[test]
    fn emtpy_email_is_rejected() {
        let email = "".to_string();
        assert!(SubscriberEmail::parse(email).is_err())
    }
    #[test]
    fn email_without_at_is_rejected() {
        let email = "testmail.rs".to_string();
        assert!(SubscriberEmail::parse(email).is_err())
    }
    #[test]
    fn email_without_subject_is_rejected() {
        let email = "@pythonruste.rs".to_string();
        assert!(SubscriberEmail::parse(email).is_err())
    }
    #[quickcheck_macros::quickcheck]
    fn valid_email_is_parsed(email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(email.0).is_ok()
    }
}
