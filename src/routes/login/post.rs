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
    appstate::AppState,
    authentication::{validate_credentials, Credentials},
};

use super::get::SignedCookieValue;

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[instrument(name = "Post Login Form", skip(form, app_state), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &app_state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok((StatusCode::SEE_OTHER, [("Location", "/")]))
        }
        Err(e) => {
            let cookie_value = SignedCookieValue::new(format!("{e}"), &app_state.hmac_secret);
            let cookie = Cookie::build(("_flash", cookie_value.into_json()))
                .http_only(true)
                .build();
            let response = Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(header::LOCATION, "/login")
                .header(header::SET_COOKIE, cookie.to_string())
                .body(Body::from("Redirecting to login"))
                .unwrap();

            Err(response)
        }
    }
}
