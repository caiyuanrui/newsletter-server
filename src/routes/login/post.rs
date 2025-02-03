use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    Form,
};
use axum_messages::Messages;
use hyper::StatusCode;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    session_state::TypedSession,
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[instrument(name = "Post Login Form", skip_all, fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    State(db_pool): State<MySqlPool>,
    messages: Messages,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            if let Err(e) = session.renew().await {
                return Err(login_redirect(anyhow::Error::from(e), messages));
            }
            if let Err(e) = session.insert_user_id(user_id).await {
                return Err(login_redirect(anyhow::Error::from(e), messages));
            }

            Ok((StatusCode::SEE_OTHER, [("Location", "/admin/dashboard")]))
        }
        Err(e) => Err(login_redirect(e, messages)),
    }
}

fn login_redirect(e: impl Into<LoginError>, messages: Messages) -> Response {
    messages.error(format!("{}", e.into()));

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login")
        .body(Body::empty())
        .unwrap()
}

#[derive(thiserror::Error, Debug)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl From<AuthError> for LoginError {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(e) => LoginError::AuthError(e),
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e),
        }
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        tracing::warn!("{:?}", self);
        match self {
            Self::AuthError(_) => StatusCode::UNAUTHORIZED.into_response(),
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
