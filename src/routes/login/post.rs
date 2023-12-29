use axum::{
    response::{ErrorResponse, Redirect},
    Form,
};
use axum_flash::Flash;
use secrecy::Secret;
use serde::Deserialize;

use crate::{
    authentication::{credentials::Credentials, middleware::AuthSession},
    routes::error_handlers::{flash_redirect, LoginError},
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, flash, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    mut session: AuthSession,
    flash: Flash,
    Form(form): Form<FormData>,
) -> axum::response::Result<Redirect> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user = match session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let err = LoginError::AuthError(anyhow::anyhow!("Unknown username."));
            return Err(login_redirect(err, flash));
        }
        Err(e) => return Err(login_redirect(LoginError::AuthError(e.into()), flash)),
    };

    session
        .login(&user)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    Ok(Redirect::to("/admin/dashboard"))
}

fn login_redirect(e: LoginError, flash: Flash) -> ErrorResponse {
    flash_redirect(&e.to_string(), "/login", flash).into()
}
