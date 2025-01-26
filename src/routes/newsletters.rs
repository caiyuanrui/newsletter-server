use std::str::FromStr;

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, Response},
    response::IntoResponse,
    Json,
};
use base64::prelude::{Engine, BASE64_STANDARD};
use hyper::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{
    appstate::AppState, domain::SubscriberEmail, domain::SubscriberId, routes::error_chain_fmt,
};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response<Body> {
        match self {
            Self::AuthError(_) => axum::http::Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header("WWW-Authenticate", r#"Basic realm="publish""#)
                .body(Body::empty())
                .expect("Failed to build http response"),
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Fetch all confirmed subscribers and send newsletters to them
#[instrument(
    name = "Publish a newsletter issue",
    skip(shared_data, body, headers),
    fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn publish_newsletter(
    State(shared_data): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<BodyData>, JsonRejection>,
) -> Result<impl IntoResponse, PublishError> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &shared_data.db_pool).await?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

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

/// Basic Authentication
/// If the API rejects the request, a response must be replied with 401 Unauthorized and includes a special header: WWW-Authenticate, containing a challenge.
struct Credentials {
    username: String,
    password: SecretString,
}

/// Authorization: Basic <encoded credentials>,
/// where <encoded credentials> is the base64-encoding of {username}:{password}
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The Authorization header is missing")?
        .to_str()
        .context("The Authorization header is not a valid UTF8 string")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The Authorization schema is not 'Base64'")?;
    let decoded_bytes = BASE64_STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials")?;
    let decoded_credentials =
        String::from_utf8(decoded_bytes).context("The decoded credential is not a valid UTF8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth"))?
        .into();

    Ok(Credentials { username, password })
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

#[instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &MySqlPool,
) -> Result<SubscriberId, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials, pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknow username")))?;

    tokio::task::spawn_blocking(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(PublishError::UnexpectedError)?
    .context("Invalid password")
    .map_err(PublishError::AuthError)?;

    Ok(user_id)
}

fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(PublishError::AuthError)
}

async fn get_stored_credentials(
    credentials: &Credentials,
    pool: &MySqlPool,
) -> Result<Option<(SubscriberId, SecretString)>, anyhow::Error> {
    sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = ?"#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|row| {
        let subscriber_id =
            SubscriberId::from_str(&row.user_id).context("Failed to parse user id")?;
        let password_hash = SecretString::new(row.password_hash.into_boxed_str());
        Ok((subscriber_id, password_hash))
    })
    .transpose()
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
