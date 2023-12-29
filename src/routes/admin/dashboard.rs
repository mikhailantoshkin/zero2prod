use anyhow::Context;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::middleware::AuthSession;

pub async fn admin_dashboard(session: AuthSession) -> axum::response::Result<Response> {
    let username = if let Some(user) = session.user {
        user.username
    } else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response().into());
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
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
    </ol>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
        <li>
            <form name="logoutForm" action="/admin/logout" method="post">
                <input type="submit" value="Logout">
            </form>
        </li>
    </ol>
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
