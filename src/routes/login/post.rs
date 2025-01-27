use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form,
};
use hyper::StatusCode;
use secrecy::SecretString;
use serde::Deserialize;
use tracing::instrument;

use crate::{
    appstate::AppState,
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[instrument(skip(form, shared_state), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    State(shared_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, LoginError> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &shared_state.db_pool).await?;

    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    Ok((StatusCode::SEE_OTHER, [("Location", "/")]))
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        match self {
            Self::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            Self::UnexpectedError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }

        // (StatusCode::SEE_OTHER, [("Location", "/")]).into_response()
    }
}

impl From<AuthError> for LoginError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(_) => Self::AuthError(value.into()),
            AuthError::UnexpectedError(_) => Self::UnexpectedError(value.into()),
        }
    }
}
