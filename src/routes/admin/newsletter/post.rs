use anyhow::{anyhow, Context};
use axum::response::Response;
use axum::Extension;
use axum::{extract::State, Form};
use axum_flash::Flash;
use sqlx::PgPool;

use crate::authentication::credentials::User;
use crate::idempotency::save_response;
use crate::idempotency::{try_processing, IdempotencyKey, NextAction};
use crate::routes::error_handlers::{e400, e500, flash_redirect};
use crate::{domain::SubscriberEmail, email_client::EmailClient};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, user, flash)
)]
pub async fn publish_newsletter(
    State(email_client): State<EmailClient>,
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    flash: Flash,
    Form(body): Form<BodyData>,
) -> axum::response::Result<Response> {
    let idempotency_key: IdempotencyKey = body.idempotency_key.try_into().map_err(e400)?;
    let tx = match try_processing(&pool, &idempotency_key, user.user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(response) => return Ok(response),
    };

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &body.title, &body.html, &body.text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?;
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
    let resp = flash_redirect(
        "The newsletter issue has been published!",
        "/admin/newsletter",
        flash,
    );
    let response = save_response(tx, &idempotency_key, user.user_id, resp)
        .await
        .map_err(e500)?;
    Ok(response)
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
