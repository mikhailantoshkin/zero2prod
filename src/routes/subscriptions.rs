use anyhow::Context;
use axum::{
    extract::{Form, State},
    http::StatusCode,
};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use uuid::Uuid;

use sqlx::{PgPool, Postgres, Transaction};

use crate::{domain::NewSubscriber, email_client::EmailClient, startup::ApplicationBaseUrl};

use super::error_handlers::PublishError;

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
) -> Result<StatusCode, PublishError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&mut transaction, &subscriber)
        .await
        .context("Failed to insert new subscriber")?;
    let token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &token)
        .await
        .context("Failed to store conformation token for a new subscriber")?;
    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;
    send_conformation_email(&email_client, subscriber, &base_url.0, &token)
        .await
        .context("Failed to send a conformation email")?;
    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Saving new subscriber to database",
    skip(transaction, subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
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
    .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(name = "Store subscription token to db", skip(transaction, token))]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
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
    .execute(&mut **transaction)
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
) -> Result<(), reqwest::Error> {
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
        .send_email(&subscriber.email, "Welcome!", html_body, text_body)
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
