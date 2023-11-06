use axum::{extract::State, response::Redirect, Form};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, Credentials},
    routes::error_handlers::LoginError,
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}
#[tracing::instrument(
    skip(form, pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> Result<Redirect, LoginError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    Ok(Redirect::to("/home"))
}
