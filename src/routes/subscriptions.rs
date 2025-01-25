use anyhow::Context;
use axum::{
    extract::{rejection::FormRejection, FromRequest, State},
    http,
    response::IntoResponse,
};
use hyper::StatusCode;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use sqlx::{MySql, Transaction};
use tracing::instrument;

use crate::{
    appstate::{AppState, ApplicationBaseUrl},
    domain::{self, NewSubscriber, SubscriberId},
    email_client::EmailClient,
};

#[axum_macros::debug_handler]
#[tracing::instrument(
  name = "Adding a new subscriber",
  skip(form, shared_state),
  fields(
  subscriber_email = %form.email,
  subscriber_name = %form.name
))]
pub async fn subscribe(
    State(shared_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<StatusCode, SubscribeError> {
    let new_subscriber = form.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = shared_state
        .db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, &subscription_token, subscriber_id)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;
    send_confirmation_email(
        &shared_state.email_client,
        &new_subscriber,
        &shared_state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(http::StatusCode::OK)
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    /// Apart from the conversion, we also log the error using the `tracing` crate.
    fn into_response(self) -> axum::response::Response {
        tracing::info!("{self:?}");
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST.into_response(),
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[tracing::instrument(
    name = "Sending a confirmation email to the subscriber",
    skip(email_client)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    base_url: &ApplicationBaseUrl,
    token: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?token={}",
        base_url.as_str(),
        token
    );
    let html_content = format!(
        r#"Welcome to our newsletter!<br />
    Click <a href="{confirmation_link}">here</a> to confirm your subscription."#,
    );
    let text_content = format!(
        r#"Welcome to our newsletter!
    Visit {confirmation_link} to confirm your subscription."#,
    );

    email_client
        .send_email(
            &new_subscriber.email,
            "Welcome",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(txn, new_subscriber)
)]
pub async fn insert_subscriber(
    txn: &mut Transaction<'_, MySql>,
    new_subscriber: &NewSubscriber,
) -> Result<SubscriberId, sqlx::Error> {
    let subscriber_id = SubscriberId::new_v4();

    sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at, status)
VALUES (?, ?, ?, ?, 'pending_confirmation')
"#,
        subscriber_id.as_str(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now()
    )
    .execute(txn.as_mut())
    .await?;

    Ok(subscriber_id)
}

#[instrument(
    name = "Store subscription token in the database",
    skip(txn, subscription_token)
)]
pub async fn store_token(
    txn: &mut Transaction<'_, MySql>,
    subscription_token: &str,
    subscriber_id: SubscriberId,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)  VALUES (?, ?)"#,
        subscription_token,
        subscriber_id.as_str()
    )
    .execute(txn.as_mut())
    .await
    .map_err(StoreTokenError)?;

    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = domain::SubscriberName::parse(form.name)?;
        let email = domain::SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { email, name })
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Form), rejection(ApiError))]
pub struct Form<T>(T);

#[derive(Debug)]
pub struct ApiError {
    status: http::StatusCode,
    message: String,
}

impl From<FormRejection> for ApiError {
    fn from(rejection: FormRejection) -> Self {
        Self {
            // failed to extract into Form
            status: if rejection.status() == http::StatusCode::UNPROCESSABLE_ENTITY {
                http::StatusCode::NOT_FOUND
            } else {
                rejection.status()
            },
            message: rejection.body_text(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let payload = json!({
          "message": self.message,
          "origin": "derive_from_request"
        });

        (self.status, axum::Json(payload)).into_response()
    }
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A databse error was encountered while trying to store a subscription token"
        )
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(source) = current {
        writeln!(f, "Caused by:\n\t{}", source)?;
        current = source.source();
    }
    Ok(())
}
