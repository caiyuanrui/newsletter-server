use axum::{extract::State, response::Response, Extension, Form};
use axum_messages::Messages;
use hyper::StatusCode;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{
    domain::{SubscriberEmail, UserId},
    email_client::EmailClient,
    idempotency::{get_saved_response, save_response, IdempotencyKey},
    utils::{e400, e500, see_other, Data},
};

/// Fetch all confirmed subscribers and send newsletters to them
#[instrument(name = "Publish a newsletter issue", skip(pool, email_client))]
pub async fn publish_newsletter(
    messages: Messages,
    State(pool): State<MySqlPool>,
    State(email_client): State<Data<EmailClient>>,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    let FormData {
        title,
        html_content,
        text_content,
        idempotency_key,
    } = form;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    // Return early if we have a saved response in the database
    if let Some(saved_response) = get_saved_response(&pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        // This is weird to resend messages by checking status code...
        // fix it later
        if saved_response.status() == StatusCode::SEE_OTHER {
            messages.success("The newsletter issue has been published!");
        }
        tracing::debug!("saved_http_response: {:?}", saved_response);
        return Ok(saved_response);
    }

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::error!(error.cause_chain = ?error, error.message = %error,
                  "Skipping a confirmed subscriber. Their stored contact details are invalid"
                );
            }
        }
    }

    let response = see_other("/admin/newsletters");
    let response = save_response(&pool, &idempotency_key, user_id, response)
        .await
        .map_err(e500)?;
    messages.success("The newsletter issue has been published!");

    Ok(response)
}

/// Get all confirmed subscribers. Filter out those subscribers with invalid email address.
#[instrument(name = "Get confirmed subscribers", skip_all)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}
