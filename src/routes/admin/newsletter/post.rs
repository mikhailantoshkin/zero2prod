use anyhow::Context;
use axum::response::Response;
use axum::Extension;
use axum::{extract::State, Form};
use axum_flash::Flash;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::authentication::credentials::User;
use crate::idempotency::save_response;
use crate::idempotency::{try_processing, IdempotencyKey, NextAction};
use crate::routes::error_handlers::{e400, e500, flash_redirect};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    html: String,
    text: String,
    idempotency_key: String,
}
#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&user.user_id)
)]
pub async fn publish_newsletter(
    State(pool): State<PgPool>,
    Extension(user): Extension<User>,
    flash: Flash,
    Form(body): Form<BodyData>,
) -> axum::response::Result<Response> {
    let idempotency_key: IdempotencyKey = body.idempotency_key.try_into().map_err(e400)?;
    let mut tx = match try_processing(&pool, &idempotency_key, user.user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(response) => return Ok(response),
    };
    let issue_id = insert_newsletter_issues(&mut tx, &body.title, &body.text, &body.html)
        .await
        .context("Fialed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut tx, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let resp = flash_redirect(
        "The newsletter issue has been accepted - emails will go out shortly.",
        "/admin/newsletter",
        flash,
    );
    let response = save_response(tx, &idempotency_key, user.user_id, resp)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issues(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        ) VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        ) SELECT $1, email FROM subscriptions WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
