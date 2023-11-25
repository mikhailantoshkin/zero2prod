use axum::{
    extract::State,
    response::{ErrorResponse, IntoResponse, Redirect},
    Form,
};
use axum_flash::Flash;
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, Credentials},
    routes::error_handlers::LoginError,
    session_state::TypedSession,
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool, flash, session),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<PgPool>,
    session: TypedSession,
    flash: Flash,
    Form(form): Form<FormData>,
) -> axum::response::Result<Redirect> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            session.renew();
            session
                .inster_user_id(user_id)
                .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into()), flash))?;
            Ok(Redirect::to("/admin/dashboard"))
        }
        Err(e) => {
            let e: LoginError = e.into();
            Err(login_redirect(e, flash))
        }
    }
}

fn login_redirect(e: LoginError, flash: Flash) -> ErrorResponse {
    (flash.error(e.to_string()), Redirect::to("/login"))
        .into_response()
        .into()
}
