use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier};

use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::prelude::FromRow;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub password_hash: Secret<String>,
}

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Option<User>, AuthError> {
    let mut authenticated_user = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
                gZiV/M1gPc22ElAH/Jh1Hw$\
                CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some(user) = get_stored_credentials(&credentials.username, pool)
        .await
        .map_err(AuthError::UnexpectedError)?
    {
        expected_password_hash = user.password_hash.clone();
        authenticated_user = Some(user);
    }

    let task_result = spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(AuthError::UnexpectedError)?;
    match task_result {
        Ok(_) => Ok(authenticated_user),
        Err(AuthError::InvalidCredentials(_)) => {
            info!("Authentifiaction failed: invalid credentials",);
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

#[tracing::instrument(name = "Verify password hash", skip(password_hash, password_candidate))]
fn verify_password_hash(
    password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(password_hash.expose_secret())
        .context("Failed to parse hash PHC string")
        .map_err(AuthError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<User>, anyhow::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT *
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve stored creds")
    .map_err(AuthError::UnexpectedError)?;
    Ok(user)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: Secret<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change users's password in the database.")?;
    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2d,
        argon2::Version::V0x13,
        Params::new(150000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();
    Ok(Secret::new(password_hash))
}

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Password has to be at least 12 symbols long.")]
    ToShort,
    #[error("Password can not be longer than 128 symblols.")]
    ToLong,
}

pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    match password.len() {
        v if v < 12 => Err(ValidationError::ToShort),
        v if v > 128 => Err(ValidationError::ToLong),
        _ => Ok(()),
    }
}
