use crate::domain::NameValidationError;
use axum::http::StatusCode;
use axum::response::IntoResponse;

impl IntoResponse for NameValidationError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, format!("{}", self)).into_response()
    }
}
