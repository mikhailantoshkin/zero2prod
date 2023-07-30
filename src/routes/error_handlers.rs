use crate::domain::NameValidationError;
use axum::http::StatusCode;
use axum::response::IntoResponse;

impl IntoResponse for NameValidationError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
    }
}

pub enum ApiError {
    DbError,
    UnexpectedError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::DbError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            ApiError::UnexpectedError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(_value: sqlx::Error) -> Self {
        ApiError::DbError
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(_value: anyhow::Error) -> Self {
        ApiError::UnexpectedError
    }
}
