use anyhow::Context;
use axum::async_trait;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum_login::AuthUser;
use axum_login::AuthnBackend;
use axum_login::UserId;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use uuid::Uuid;

use super::credentials::validate_credentials;
use super::credentials::AuthError;
use super::credentials::Credentials;
use super::credentials::User;

impl AuthUser for User {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.user_id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.get_password_hash().expose_secret().as_bytes()
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
    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let user = sqlx::query_as(
            r#"
        SELECT *
        FROM users
        WHERE user_id = $1
        "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to retrieve stored creds")
        .map_err(AuthError::UnexpectedError)?;

        Ok(user)
    }
}

pub async fn auth_middleware(
    auth_session: AuthSession,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(user) = auth_session.user {
        let span = tracing::Span::current();
        span.record("username", &tracing::field::display(&user.username));
        span.record("user_id", &tracing::field::display(&user.user_id));
        request.extensions_mut().insert(user);
        next.run(request).await
    } else {
        Redirect::to("/login").into_response()
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
