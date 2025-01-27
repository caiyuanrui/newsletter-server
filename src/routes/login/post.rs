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
use tracing::instrument;

use crate::{
    appstate::AppState,
    authentication::{validate_credentials, Credentials},
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[instrument(name = "Post Login Form", skip(form, shared_state), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    State(shared_state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &shared_state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            Ok((StatusCode::SEE_OTHER, [("Location", "/")]))
        }
        Err(e) => {
            let response = Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(header::LOCATION, "/login")
                .header(header::SET_COOKIE, format!("_flash={e}"))
                .body(Body::from("Redirecting to login"))
                .unwrap();

            Err(response)
        }
    }
}
