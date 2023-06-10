use axum::{
    extract::{Form, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Subscriber {
    name: String,
    email: String,
}

#[tracing::instrument(
    name="Adding a new subscriber", 
    skip(pool, subscriber),
    fields(
        subscriber_email = %subscriber.email,
        subscriber_name=subscriber.name
    )
)]
pub async fn subscribe(
    State(pool): State<PgPool>,
    Form(subscriber): Form<Subscriber>,
) -> StatusCode {
    let result = inster_subscriber(&pool, &subscriber).await;
    match result {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "Saving new subscriber to database", skip(pool, subscriber))]
pub async fn inster_subscriber(pool: &PgPool, subscriber: &Subscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        subscriber.email,
        subscriber.name,
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;
    Ok(())
}
