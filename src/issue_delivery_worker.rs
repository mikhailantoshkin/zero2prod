use std::time::Duration;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Failed to deliver issue to a confirmed subscriber")]
    SendingError(#[source] anyhow::Error),
    #[error("Subscriber stored contact deatails are invalid")]
    WrongCredentials(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
    ),
    err
)]
pub async fn try_execute_taks(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, ExecutionError> {
    let task = dequeue_taks(pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (mut tx, issue_id, email) = task.unwrap();
    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));
    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue = get_issue(&mut tx, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. Skipping",
                );
                requeue_taks(tx, issue_id, email.as_ref()).await?;
                return Err(ExecutionError::SendingError(e.into()));
            }
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. Their stored contact deatails are invalid",
            );
            delete_tasks_for_subscriber(tx, &email).await?;
            return Err(ExecutionError::WrongCredentials(email));
        }
    }
    delete_task(tx, issue_id, &email).await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database).await?;
    let email_client = configuration.email_client.client();
    worker_loop(&connection_pool, email_client).await
}

async fn worker_loop(pool: &PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_taks(pool, &email_client).await {
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

#[tracing::instrument(skip_all)]
async fn dequeue_taks(
    pool: &PgPool,
) -> Result<Option<(Transaction<'_, Postgres>, Uuid, String)>, anyhow::Error> {
    let mut tx = pool.begin().await?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        WHERE next_retry <= now()
        ORDER BY next_retry ASC
        FOR UPDATE
        SKIP LOCKED
        LIMIT(1)
        "#
    )
    .fetch_optional(&mut *tx)
    .await?;
    Ok(r.map(|rec| (tx, rec.newsletter_issue_id, rec.subscriber_email)))
}

#[tracing::instrument(skip_all)]
async fn requeue_taks(
    mut tx: Transaction<'_, Postgres>,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE issue_delivery_queue
        SET next_retry = now() + interval '1 seconds'
        WHERE
            newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email
    )
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut tx: Transaction<'_, Postgres>,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn delete_tasks_for_subscriber(
    mut tx: Transaction<'_, Postgres>,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE
            subscriber_email = $1
        "#,
        email
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[allow(dead_code)]
struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(
    tx: &mut Transaction<'_, Postgres>,
    issue_id: Uuid,
) -> Result<NewsletterIssue, anyhow::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
            newsletter_issue_id = $1
        "#,
        issue_id
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(issue)
}
