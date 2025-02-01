use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    Form,
};
use hyper::StatusCode;
use secrecy::SecretString;
use serde::Deserialize;
use tower_cookies::Cookie;
use tracing::instrument;

use crate::{
    appstate::{AppState, HmacSecret},
    authentication::{validate_credentials, AuthError, Credentials},
    session_state::TypedSession,
};

use super::get::SignedCookieValue;

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[instrument(name = "Post Login Form", skip(form, app_state, session), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    State(app_state): State<AppState>,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &app_state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));

            session
                .renew()
                .await
                .map_err(|e| login_redirect(anyhow::Error::from(e), &app_state.hmac_secret))?;
            session
                .insert_user_id(user_id)
                .await
                .map_err(|e| login_redirect(anyhow::Error::from(e), &app_state.hmac_secret))?;

            Ok((StatusCode::SEE_OTHER, [("Location", "/admin/dashboard")]))
        }
        Err(e) => Err(login_redirect(e, &app_state.hmac_secret)),
    }
}

fn login_redirect(e: impl Into<LoginError>, secret: &HmacSecret) -> Response {
    let e = e.into();
    let cookie_value = SignedCookieValue::new(format!("{e}"), secret);
    let cookie = Cookie::build(("_flash", cookie_value.into_json()))
        .http_only(true)
        .build();
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(Body::from(format!(
            "Error occurs: {e}.Redirecting to login"
        )))
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
