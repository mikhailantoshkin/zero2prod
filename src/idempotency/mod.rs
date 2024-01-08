mod persistence;

pub use persistence::*;

#[derive(Debug)]
pub struct IdempotencyKey(String);

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.len() {
            0 => anyhow::bail!("The idempotency key cannot be empty"),
            v if v >= 50 => anyhow::bail!("The idempotency key must be shorter than 50 characters"),
            _ => Ok(Self(value)),
        }
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
