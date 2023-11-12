// TODO: remove this with new release of axum-extra
use axum::{
    async_trait,
    extract::{rejection::QueryRejection, FromRequestParts, Query},
};
use http::request::Parts;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, Default)]
pub struct OptionalQuery<T>(pub Option<T>);

#[async_trait]
impl<T, S> FromRequestParts<S> for OptionalQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if parts.uri.query().is_none() {
            return Ok(Self(None));
        }
        let q = Query::try_from_uri(&parts.uri)?;
        Ok(Self(Some(q.0)))
    }
}
