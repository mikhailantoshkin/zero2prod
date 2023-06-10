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

pub async fn subscriptions(
    State(pool): State<PgPool>,
    Form(subscriber): Form<Subscriber>,
) -> StatusCode {
    let result = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        subscriber.email,
        subscriber.name,
        Utc::now(),
    )
    .execute(&pool)
    .await;
    match result {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Query execution failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
