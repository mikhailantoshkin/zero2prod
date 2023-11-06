use axum::extract::{Query, State};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::error_handlers::PublishError;
use anyhow::Context;

#[derive(Deserialize)]
pub struct QueryParams {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, params))]
pub async fn subscribtion_confirm(
    State(pool): State<PgPool>,
    Query(params): Query<QueryParams>,
) -> Result<StatusCode, PublishError> {
    let id = get_subscriber_id_by_token(&pool, &params.subscription_token)
        .await
        .context("Failed to aquire connection from the db pool")?;
    match id {
        None => return Ok(StatusCode::UNAUTHORIZED),
        Some(subscriber_id) => confirm_subscriber(&pool, subscriber_id)
            .await
            .context("Failed to confirm subscriber")?,
    }
    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Get subscriber id from db", skip(pool, token))]
pub async fn get_subscriber_id_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}
