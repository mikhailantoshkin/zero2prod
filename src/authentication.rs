use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};

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
    password_hash: Secret<String>,
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

pub mod middleware {
    use axum::async_trait;
    use axum::extract::Request;
    use axum::middleware::Next;
    use axum::response::IntoResponse;
    use axum::response::Redirect;
    use axum::response::Response;
    use axum_login::AuthUser;
    use axum_login::AuthnBackend;
    use axum_login::UserId;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    impl AuthUser for User {
        type Id = Uuid;

        fn id(&self) -> Self::Id {
            self.user_id
        }

        fn session_auth_hash(&self) -> &[u8] {
            self.password_hash.expose_secret().as_bytes()
        }
    }

    #[derive(Debug, Clone)]
    pub struct Backend {
        db: PgPool,
    }

    impl Backend {
        pub fn new(db: PgPool) -> Self {
            Self { db }
        }
    }

    #[async_trait]
    impl AuthnBackend for Backend {
        type User = User;
        type Credentials = Credentials;
        type Error = AuthError;

        async fn authenticate(
            &self,
            creds: Self::Credentials,
        ) -> Result<Option<Self::User>, Self::Error> {
            validate_credentials(creds, &self.db).await
        }
        async fn get_user(
            &self,
            user_id: &UserId<Self>,
        ) -> Result<Option<Self::User>, Self::Error> {
            let user = sqlx::query_as!(
                User,
                r#"
        SELECT *
        FROM users
        WHERE user_id = $1
        "#,
                user_id,
            )
            .fetch_optional(&self.db)
            .await
            .context("Failed to retrieve stored creds")
            .map_err(AuthError::UnexpectedError)?;

            Ok(user)
        }
    }

    pub async fn auth_middleware(
        auth_session: AuthSession,
        request: Request,
        next: Next,
    ) -> Response {
        if auth_session.user.is_some() {
            next.run(request).await
        } else {
            Redirect::to("/login").into_response()
        }
    }
}

pub type AuthSession = axum_login::AuthSession<middleware::Backend>;
