use std::fmt::Debug;

use crate::authentication::AuthError;
use crate::domain::NameValidationError;
use axum::http::header;
use axum::http::HeaderValue;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use hyper::HeaderMap;

impl IntoResponse for NameValidationError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
    }
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<AuthError> for PublishError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(value.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(value.into()),
        }
    }
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Error in handler: {:?}", self);
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            PublishError::AuthError(_) => {
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap(),
                );
                (StatusCode::UNAUTHORIZED, headers).into_response()
            }
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LoginError::AuthError(_) => Redirect::to("/login").into_response(),
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl From<AuthError> for LoginError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(value.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(value.into()),
        }
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by: \n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
