use std::fmt::{Debug, Display};

use crate::domain::NameValidationError;
use axum::http::StatusCode;
use axum::response::IntoResponse;

impl IntoResponse for NameValidationError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
    }
}

#[derive(thiserror::Error)]
pub enum AppError {
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unexpected backend error")
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Error in handler: {:?}", self);
        match self {
            AppError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
