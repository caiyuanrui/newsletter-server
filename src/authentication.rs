use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use std::str::FromStr;

use super::{domain::SubscriberId, telementry::spawn_blocking_with_tracing};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Basic Authentication
/// If the API rejects the request, a response must be replied with 401 Unauthorized and includes a special header: WWW-Authenticate, containing a challenge.
#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &MySqlPool,
) -> Result<SubscriberId, AuthError> {
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials, pool)
        .await
        .map_err(AuthError::UnexpectedError)?
        .ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknow username")))?;

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task")
    .map_err(AuthError::UnexpectedError)??;

    Ok(user_id)
}

#[instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[instrument(name = "Get stored credentials", skip(credentials, pool))]
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
