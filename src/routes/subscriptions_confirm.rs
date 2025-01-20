use axum::{
    extract::{rejection::QueryRejection, Query, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{appstate::AppState, domain::SubscriberId};

#[instrument(name = "Confirm a pending subscriber", skip(params, shared_state))]
pub async fn confirm(
    params: Result<Query<Params>, QueryRejection>,
    shared_state: State<AppState>,
) -> impl IntoResponse {
    match params {
        Ok(Query(params)) => {
            let subscriber_id =
                match get_subscriber_id_with_token(&params.token, &shared_state.db_pool).await {
                    Ok(id) => id,
                    Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
                };

            match subscriber_id {
                Some(subscriber_id) => {
                    if confirm_subscriber(&shared_state.db_pool, subscriber_id)
                        .await
                        .is_err()
                    {
                        return StatusCode::INTERNAL_SERVER_ERROR;
                    }
                }
                None => return StatusCode::UNAUTHORIZED,
            }

            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("{e}");
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
) -> Result<Option<SubscriberId>, sqlx::Error> {
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
        r.subscriber_id.as_str().try_into().expect(
            "Failed to parse the uuid fetched from the databse! Check the schema consistency please!",
        )
    }))
}

#[instrument(name = "Make the subscriber status as confirmed", skip(pool, id))]
async fn confirm_subscriber(pool: &MySqlPool, id: SubscriberId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = ?"#,
        id.into_string(),
    )
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e}");
        e
    })
}
