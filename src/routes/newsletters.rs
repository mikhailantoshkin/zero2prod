use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::State, Json};

use axum::http::{HeaderMap, StatusCode};
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::spawn_blocking_with_tracing;
use crate::{domain::SubscriberEmail, email_client::EmailClient};

use super::error_handlers::AppError;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[tracing::instrument(
    name="Publish a newsletter issue",
    skip(body, pool, email_client, headers),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    headers: HeaderMap,
    State(email_client): State<EmailClient>,
    State(pool): State<PgPool>,
    Json(body): Json<BodyData>,
) -> Result<StatusCode, AppError> {
    let creds = basic_auth(&headers).map_err(AppError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&creds.username));
    let user_id = validate_credentials(creds, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Teir stored contact details are invalid",
                )
            }
        }
    }
    Ok(StatusCode::OK)
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

fn basic_auth(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_val = headers
        .get("Authorization")
        .context("No Auth header present")?
        .to_str()
        .context("Auth header is not UTF-8")?;
    let b64_segment = header_val
        .strip_prefix("Basic ")
        .context("Auth is not Basi")?;
    let decoded = base64::decode(b64_segment).context("Unable to decode Basic creds")?;
    let decoded_creds = String::from_utf8(decoded).context("Credentials are not valid UTF8")?;
    let mut credentials = decoded_creds.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<Uuid, AppError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
                gZiV/M1gPc22ElAH/Jh1Hw$\
                CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, password_hash)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(AppError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(AppError::UnexpectedError)??;
    user_id.ok_or_else(|| AppError::AuthError(anyhow::anyhow!("Uknown username.")))
}

#[tracing::instrument(name = "Verify password hash", skip(password_hash, password_candidate))]
fn verify_password_hash(
    password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AppError> {
    let expected_password_hash = PasswordHash::new(password_hash.expose_secret())
        .context("Failed to parse hash PHC string")
        .map_err(AppError::UnexpectedError)?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Ivalid password")
        .map_err(AppError::AuthError)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retreieve stored creds")
    .map_err(AppError::UnexpectedError)?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
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
