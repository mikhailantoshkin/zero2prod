use anyhow::{anyhow, Context};
use axum::http::StatusCode;
use axum::{extract::State, Form};
use sqlx::PgPool;

use crate::routes::error_handlers::PublishError;
use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
}

#[tracing::instrument(name = "Publish a newsletter issue", skip(body, pool, email_client))]
pub async fn publish_newsletter(
    State(email_client): State<EmailClient>,
    State(pool): State<PgPool>,
    Form(body): Form<BodyData>,
) -> Result<StatusCode, PublishError> {
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &body.title, &body.html, &body.text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                )
            }
        }
    }
    Ok(StatusCode::OK)
}
struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#,)
        .fetch_all(pool)
        .await?;
    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow!(error)),
        })
        .collect();
    Ok(confirmed_subscribers)
}
