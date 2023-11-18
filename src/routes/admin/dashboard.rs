use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::Html};
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

pub async fn admin_dashboard(
    State(pool): State<PgPool>,
    session: Session,
) -> axum::response::Result<Html<String>> {
    let username = if let Some(user_id) = session
        .get::<Uuid>("user_id")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        get_username(user_id, &pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        todo!()
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
    )))
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
