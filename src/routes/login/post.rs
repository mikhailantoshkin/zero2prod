use axum::{extract::State, response::Redirect, Form};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
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
    jar: SignedCookieJar,
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
            Ok(Redirect::to("/home"))
        }
        Err(e) => {
            let e: LoginError = e.into();
            let jar = jar.add(Cookie::new("_flash", e.to_string()));
            Err((jar, Redirect::to("/login")))?
        }
    }
}
