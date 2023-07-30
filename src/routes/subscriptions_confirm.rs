use axum::extract::{Query, State};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::error_handlers::ApiError;

#[derive(Deserialize)]
pub struct QueryParams {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, params))]
pub async fn subscribtion_confirm(
    State(pool): State<PgPool>,
    Query(params): Query<QueryParams>,
) -> Result<StatusCode, ApiError> {
    let id = get_subscriber_id_by_token(&pool, &params.subscription_token).await?;
    match id {
        None => return Ok(StatusCode::UNAUTHORIZED),
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_err() {
                return Ok(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
