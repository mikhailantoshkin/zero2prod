use axum::{
    extract::State,
    http::StatusCode,
    response::{ErrorResponse, IntoResponse, Response},
    Form,
};
use axum_flash::Flash;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::authentication::{
    credentials::{
        change_password as change_pass, validate_credentials, validate_password, Credentials,
    },
    middleware::AuthSession,
};
use crate::routes::error_handlers::flash_redirect;

#[derive(thiserror::Error, Debug)]
enum PasswordError {
    #[error("Something unexpcted happend")]
    UnexpectedError(#[from] anyhow::Error),
}
impl IntoResponse for PasswordError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PasswordError::UnexpectedError(e) => {
                tracing::error!("Error during password change {}", e.to_string());
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    State(pool): State<PgPool>,
    session: AuthSession,
    flash: Flash,
    Form(form): Form<FormData>,
) -> Result<Response, ErrorResponse> {
    if let Err(err) = validate_password(form.new_password.expose_secret()) {
        return Err(as_flash(&err.to_string(), flash).into());
    };
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        return Err(as_flash(
            "You entered two different new passwords - the field values must match.",
            flash,
        )
        .into());
    }
    let user = session.user.as_ref().unwrap();
    let credentials = Credentials {
        username: user.username.clone(),
        password: form.current_password,
    };
    if (validate_credentials(credentials, &pool)
        .await
        .map_err(|e| PasswordError::UnexpectedError(e.into()))?)
    .is_none()
    {
        return Err(as_flash("The current password is incorrect.", flash).into());
    };
    // this will invalidate the session, since password hash has changed and it is used as session id
    // TODO: don't invalidate the session on password change
    change_pass(user.user_id, form.new_password, &pool)
        .await
        .map_err(PasswordError::UnexpectedError)?;
    Ok(as_flash("Your password has been changed.", flash))
}

fn as_flash(msg: &str, flash: Flash) -> Response {
    flash_redirect(msg, "/admin/password", flash)
}
