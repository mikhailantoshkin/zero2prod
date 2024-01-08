use super::IdempotencyKey;
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::Response;
use http::HeaderName;
use sqlx::postgres::PgHasArrayType;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn get_seved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT 
            response_status_code,
            response_headers as "response_headers: Vec<HeaderPairRecord>",
            response_body
        FROM idempotency
        WHERE
            user_id = $1
            AND idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;
    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut builder = Response::builder().status(status_code);
        let headers = builder.headers_mut().unwrap();
        for HeaderPairRecord { name, value } in r.response_headers {
            headers.append(
                HeaderName::from_bytes(name.as_bytes())?,
                value.as_slice().try_into()?,
            );
        }
        let resp = builder.body(r.response_body.into())?;
        Ok(Some(resp))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: Response,
) -> Result<Response, anyhow::Error> {
    let (parts, body) = http_response.into_parts();
    let body = to_bytes(body, usize::MAX).await?;
    let status_code = parts.status.as_u16() as i16;

    let headers: Vec<HeaderPairRecord> = parts
        .headers
        .iter()
        .map(|(name, value)| HeaderPairRecord {
            name: name.as_str().to_owned(),
            value: value.as_bytes().to_owned(),
        })
        .collect();

    sqlx::query_unchecked!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            response_status_code,
            response_headers,
            response_body,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, now())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref()
    )
    .execute(pool)
    .await?;

    let http_response = Response::from_parts(parts, body.into());
    Ok(http_response)
}
