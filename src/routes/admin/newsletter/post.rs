use axum::{extract::State, response::Response, Form};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{
    domain::SubscriberEmail,
    email_client::EmailClient,
    utils::{e500, see_other, Data},
};

/// Fetch all confirmed subscribers and send newsletters to them
#[instrument(name = "Publish a newsletter issue", skip_all)]
pub async fn publish_newsletter(
    messages: Messages,
    State(db_pool): State<MySqlPool>,
    State(email_client): State<Data<EmailClient>>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    let subscribers = get_confirmed_subscribers(&db_pool).await.map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &form.title,
                        &form.html_content,
                        &form.text_content,
                    )
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

    messages.success("The newsletter issue has been published!");
    Ok(see_other("/admin/newsletters"))
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
}
