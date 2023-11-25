use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;

pub async fn admin_dashboard(
    State(pool): State<PgPool>,
    session: TypedSession,
) -> axum::response::Result<Response> {
    let username = if let Some(user_id) = session
        .get_user_id()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        get_username(user_id, &pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        return Ok(Redirect::to("/login").into_response());
    };
    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
</body>
</html>"#
    ))
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
SELECT username
FROM users
WHERE user_id = $1
"#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
