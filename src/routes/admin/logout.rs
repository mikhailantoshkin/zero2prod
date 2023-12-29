use crate::{authentication::middleware::AuthSession, routes::error_handlers::flash_redirect};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_flash::Flash;

pub async fn log_out(flash: Flash, mut session: AuthSession) -> axum::response::Result<Response> {
    session.logout().await.map_err(|e| {
        tracing::error!("Error during logout: {}", e.to_string());
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;
    Ok(flash_redirect(
        "You have successfully logged out.",
        "/login",
        flash,
    ))
}
