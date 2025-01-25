use anyhow::Context;
use axum::{
    extract::{rejection::JsonRejection, State},
    response::IntoResponse,
    Json,
};
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{appstate::AppState, domain::SubscriberEmail, routes::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

/// Fetch all confirmed subscribers and send newsletters to them
#[instrument(name = "Dummy implementation of newsletter", skip(shared_data, body))]
pub async fn publish_newsletter(
    State(shared_data): State<AppState>,
    body: Result<Json<BodyData>, JsonRejection>,
) -> Result<impl IntoResponse, PublishError> {
    let body = match body {
        Ok(Json(body)) => body,
        Err(e) => {
            tracing::warn!("Failed to extract json body: {}", e);
            return Ok(StatusCode::BAD_REQUEST);
        }
    };

    let subscribers = get_confirmed_subscribers(&shared_data.db_pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                shared_data
                    .email_client
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
                tracing::warn!(error.cause_chain = ?error,
                  "Skipping a confirmed subscriber. \
                  Their stored contact details are invalid"
                );
            }
        }
    }

    Ok(StatusCode::OK)
}

/// Get all confirmed subscribers. Filter out those subscribers with invalid email address.
#[instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &MySqlPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
      SELECT email
      FROM subscriptions
      WHERE status = 'confirmed'
      "#
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}
