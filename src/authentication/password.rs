use anyhow::Context;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use std::str::FromStr;

use crate::{domain::UserId, telementry::spawn_blocking_with_tracing};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed")]
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
) -> Result<UserId, AuthError> {
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
) -> Result<Option<(UserId, SecretString)>, anyhow::Error> {
    sqlx::query!(
        r#"SELECT user_id, password_hash FROM users WHERE username = ?"#,
        credentials.username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials")?
    .map(|row| {
        let subscriber_id = UserId::from_str(&row.user_id).context("Failed to parse user id")?;
        let password_hash = SecretString::new(row.password_hash.into_boxed_str());
        Ok((subscriber_id, password_hash))
    })
    .transpose()
}

pub async fn change_password(
    user_id: UserId,
    password: SecretString,
    pool: &MySqlPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
      UPDATE users
      SET password_hash = ?
      WHERE user_id = ?
      "#,
        password_hash.expose_secret(),
        user_id.to_string()
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the databse")?;

    Ok(())
}

fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(password.expose_secret().as_bytes(), salt.as_salt())?
        .to_string();
    Ok(SecretString::from(password_hash))
}
