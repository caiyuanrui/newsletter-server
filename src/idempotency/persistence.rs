use anyhow::Context;
use axum::{body::to_bytes, response::Response};
use hyper::StatusCode;
use sqlx::{MySqlPool, MySqlTransaction};
use tracing::instrument;

use crate::domain::UserId;

use super::{HeaderPairRecord, Headers, IdempotencyKey};

/// Commit transaction if successfully update the idempotency table
#[instrument(name = "Save response to database")]
pub async fn save_response(
    mut txn: MySqlTransaction<'static>,
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
      UPDATE idempotency
      SET
      response_status_code = ?,
      response_headers = ?,
      response_body = ?
      WHERE
      user_id = ? AND
      idempotency_key = ?
      "#,
        status_code,
        headers_bytes,
        bytes.as_ref(),
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(&mut *txn)
    .await
    .context("Failed to execute query")?;
    txn.commit().await?;

    let response = Response::from_parts(parts, bytes.into());
    Ok(response)
}

#[instrument(name = "Get saved resposne from database")]
pub async fn get_saved_response(
    pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query_unchecked!(
        r#"
      SELECT
        response_status_code as "response_status_code!",
        response_headers as "response_headers!",
        response_body as "response_body!"
      FROM idempotency
      WHERE
        user_id = ? AND
        idempotency_key = ?
      "#,
        user_id,
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

pub enum NextAction {
    StartProcessing(sqlx::MySqlTransaction<'static>),
    ReturnSavedResponse(Response),
}

/// The transaction starts here.
/// If you're using Postgres, make sure that the isolation level is read committed,
/// otherwise, the serialization will fail.
pub async fn try_processing(
    pool: &MySqlPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) -> Result<NextAction, anyhow::Error> {
    let mut txn = pool.begin().await?;

    let n_inserted_rows = sqlx::query!(
        r#"
      INSERT IGNORE INTO idempotency(
      user_id,
      idempotency_key,
      created_at
      )
      VALUES (?, ?, now())
      "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(txn))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
