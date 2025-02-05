use std::ops::{Deref, DerefMut};

use anyhow::Context;
use axum::{body::to_bytes, response::Response};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use tracing::instrument;

use crate::domain::UserId;

use super::IdempotencyKey;

#[instrument(name = "Save response to database")]
pub async fn save_response(
    pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
    response: Response,
) -> Result<Response, anyhow::Error> {
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX)
        .await
        .context("HTTP body's size exceeds the limitation")?;
    let status_code = parts.status.as_u16() as i16;
    let headers: Headers = parts
        .headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_bytes()).into())
        .collect();
    let headers_bytes: Vec<u8> = headers
        .try_into()
        .context("Failed to convert headers into bytes")?;

    sqlx::query!(
        r#"
      INSERT INTO idempotency (
        user_id,
        idempotency_key,
        response_status_code,
        response_headers,
        response_body,
        created_at
      )
      VALUES (?, ?, ?, ?, ?, now())
      "#,
        user_id.to_string(),
        idempotency_key.as_ref(),
        status_code,
        headers_bytes,
        bytes.as_ref(),
    )
    .execute(pool)
    .await
    .context("Failed to execute query")?;

    let response = Response::from_parts(parts, bytes.into());
    Ok(response)
}

#[instrument(name = "Get saved resposne from database")]
pub async fn get_saved_response(
    pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
      SELECT
        response_status_code,
        response_headers,
        response_body
      FROM idempotency
      WHERE
        user_id = ? AND
        idempotency_key = ?
      "#,
        user_id.to_string(),
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code as u16)?;
        let mut response = Response::builder().status(status_code);
        let headers: Headers = r.response_headers.as_slice().try_into()?;
        for HeaderPairRecord { key, value } in headers {
            response = response.header(key, value);
        }
        Ok(Some(response.body(r.response_body.into())?))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderPairRecord {
    pub key: String,
    pub value: Vec<u8>,
}

impl<T, U> From<(T, U)> for HeaderPairRecord
where
    T: Into<String>,
    U: Into<Vec<u8>>,
{
    fn from(value: (T, U)) -> Self {
        Self {
            key: value.0.into(),
            value: value.1.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub Vec<HeaderPairRecord>);

impl Headers {
    pub fn new<T, U>(inner: T) -> Self
    where
        T: Into<Vec<U>>,
        U: Into<HeaderPairRecord>,
    {
        Self(inner.into().into_iter().map(Into::into).collect())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.try_into().expect("Failed to serialize Headers")
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bytes.try_into().expect("Failed to deserialize Headers")
    }
}

impl TryFrom<&[u8]> for Headers {
    type Error = bincode::Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes)
    }
}

impl TryFrom<&Headers> for Vec<u8> {
    type Error = bincode::Error;
    fn try_from(value: &Headers) -> Result<Self, Self::Error> {
        bincode::serialize(value)
    }
}

impl TryFrom<Headers> for Vec<u8> {
    type Error = bincode::Error;
    fn try_from(value: Headers) -> Result<Self, Self::Error> {
        bincode::serialize(&value)
    }
}

impl From<&axum::http::HeaderMap> for Headers {
    fn from(value: &axum::http::HeaderMap) -> Self {
        value
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()).into())
            .collect()
    }
}

impl From<axum::http::HeaderMap> for Headers {
    fn from(value: axum::http::HeaderMap) -> Self {
        value
            .into_iter()
            .filter_map(|(name, value)| {
                name.map(|name| HeaderPairRecord {
                    key: name.to_string(),
                    value: value.as_bytes().to_vec(),
                })
            })
            .collect()
    }
}

impl Deref for Headers {
    type Target = Vec<HeaderPairRecord>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Headers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<HeaderPairRecord> for Headers {
    fn from_iter<T: IntoIterator<Item = HeaderPairRecord>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for Headers {
    type Item = HeaderPairRecord;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_headers_as_bytes() {
        let raw_headers = Headers::new([
            ("Content-Type", "application/json"),
            ("Content_length", "0"),
        ]);
        let bytes = raw_headers.to_bytes();
        let new_headers = Headers::from_bytes(&bytes);
        assert_eq!(raw_headers, new_headers);
    }
}
