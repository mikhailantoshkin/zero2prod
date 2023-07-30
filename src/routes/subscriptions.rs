use axum::{
    extract::{Form, State},
    http::StatusCode,
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use uuid::Uuid;

use sqlx::PgPool;

use crate::{domain::NewSubscriber, email_client::EmailClient, startup::ApplicationBaseUrl};

use super::error_handlers::ApiError;

#[tracing::instrument(
    name="Adding a new subscriber", 
    skip(pool, email_client, base_url, subscriber),
    fields(
        subscriber_email = %subscriber.email.as_ref(),
        subscriber_name=subscriber.name.as_ref()
    )
)]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<ApplicationBaseUrl>,
    Form(subscriber): Form<NewSubscriber>,
) -> Result<StatusCode, ApiError> {
    let subscriber_id = inster_subscriber(&pool, &subscriber).await?;
    let token = generate_subscription_token();
    store_token(&pool, subscriber_id, &token).await?;
    send_conformation_email(&email_client, subscriber, &base_url.0, &token).await?;
    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Saving new subscriber to database", skip(pool, subscriber))]
pub async fn inster_subscriber(
    pool: &PgPool,
    subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_conformation')
    "#,
        subscriber_id,
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
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Store subscription token to db", skip(pool, token))]
pub async fn store_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        token,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
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
    base_url: &str,
    conformation_token: &str,
) -> Result<(), anyhow::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, conformation_token
    );
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

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
