use axum::{
    extract::{Form, State},
    http::StatusCode,
};
use chrono::Utc;
use uuid::Uuid;

use sqlx::PgPool;

use crate::{domain::NewSubscriber, email_client::EmailClient};

#[tracing::instrument(
    name="Adding a new subscriber", 
    skip(pool,email_client, subscriber),
    fields(
        subscriber_email = %subscriber.email.as_ref(),
        subscriber_name=subscriber.name.as_ref()
    )
)]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Form(subscriber): Form<NewSubscriber>,
) -> StatusCode {
    if inster_subscriber(&pool, &subscriber).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    if send_conformation_email(&email_client, subscriber)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

#[tracing::instrument(name = "Saving new subscriber to database", skip(pool, subscriber))]
pub async fn inster_subscriber(
    pool: &PgPool,
    subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_conformation')
    "#,
        Uuid::new_v4(),
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
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

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, subscriber)
)]
pub async fn send_conformation_email(
    email_client: &EmailClient,
    subscriber: NewSubscriber,
) -> Result<(), anyhow::Error> {
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
    let text_body = &format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = &format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(subscriber.email, "Welcome!", html_body, text_body)
        .await
}
