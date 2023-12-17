use std::{path::PathBuf, time::Duration};

use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Deserializer};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};

use crate::domain::SubscriberEmail;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app: AppSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: Secret<String>,
}

#[derive(Deserialize, Clone)]
pub struct AppSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_key_secret")]
    pub hmac_secret: Vec<u8>,
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: SubscriberEmail,
    pub authorization_token: Secret<String>,
    #[serde(deserialize_with = "deserialize_duration_from_millis")]
    pub timeout_millis: Duration,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .username(&self.username)
            .password(self.password.expose_secret())
            .host(&self.host)
            .ssl_mode(ssl_mode)
            .port(self.port)
    }
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.database_name)
            .log_statements(tracing::log::LevelFilter::Trace)
    }
}

pub enum Environment {
    Local,
    Production,
}
impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!(
                "Unsupported environment type: {}. Use `local` or `production`",
                other
            )),
        }
    }
}

fn deserialize_duration_from_millis<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millis = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(millis))
}

fn deserialize_key_secret<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let secret: Vec<u8> = String::deserialize(deserializer)?.into();
    if secret.len() < 32 {
        return Err(serde::de::Error::custom(
            "Secret string must be at least 32 bytes long",
        ));
    }

    Ok(secret)
}

pub fn get_config() -> Result<Settings, config::ConfigError> {
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse API_ENVIRONMENT");
    let config_dir: PathBuf = std::env::var("CONFIG_DIR")
        .unwrap_or_else(|_| match environment {
            Environment::Local => "configuration".into(),
            Environment::Production => "/etc/zero2prod".into(),
        })
        .into();
    config::Config::builder()
        .add_source(config::File::from(config_dir.join("base")).required(true))
        .add_source(config::File::from(config_dir.join(environment.as_str())).required(true))
        .add_source(config::Environment::with_prefix("app"))
        .build()?
        .try_deserialize()
}
