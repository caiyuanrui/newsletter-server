use axum::{
    extract::{rejection::QueryRejection, Query, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::domain::UserId;

#[instrument(name = "Confirm a pending subscriber", skip(params, db_pool))]
pub async fn confirm(
    params: Result<Query<Params>, QueryRejection>,
    State(db_pool): State<MySqlPool>,
) -> impl IntoResponse {
    match params {
        Ok(Query(params)) => {
            let subscriber_id = match get_subscriber_id_with_token(&params.token, &db_pool).await {
                Ok(id) => id,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
            };

            match subscriber_id {
                Some(subscriber_id) => {
                    if confirm_subscriber(&db_pool, subscriber_id).await.is_err() {
                        return StatusCode::INTERNAL_SERVER_ERROR;
                    }
                }
                None => return StatusCode::UNAUTHORIZED,
            }

            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("Failed to parse query: {e}");
            StatusCode::BAD_REQUEST
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub token: String,
}

async fn get_subscriber_id_with_token(
    token: &str,
    pool: &MySqlPool,
) -> Result<Option<UserId>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = ?"#,
        token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e}");
        e
    })?;

    Ok(result.map(|r| {
        UserId::from_slice(&r.subscriber_id).expect("Failed to parse subscriber_id into uuid")
    }))
}

#[instrument(name = "Make the subscriber status as confirmed", skip(pool, id))]
async fn confirm_subscriber(pool: &MySqlPool, id: UserId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = ?"#,
        id,
    )
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e}");
        e
    })
}
