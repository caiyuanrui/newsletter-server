use axum::{extract::State, response::IntoResponse, Form};
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
            // let error_message = e.to_string();
            // let error_message = urlencoding::encode(&error_message);
            // let secret: &[u8] = shared_state.hmac_secret.expose_secret().as_bytes();
            // let hmac_tag = {
            //     let mut mac = Hmac::<sha3::Sha3_256>::new_from_slice(secret).unwrap();
            //     mac.update(error_message.as_bytes());
            //     mac.finalize().into_bytes()
            // };

            Err((
                StatusCode::SEE_OTHER,
                [("Location", "/login")],
                e.to_string(),
            ))
        }
    }
}

// #[derive(thiserror::Error)]
// pub enum LoginError {
//     #[error("Authentication failed")]
//     AuthError(#[source] anyhow::Error),
//     #[error("Something went wrong")]
//     UnexpectedError(#[from] anyhow::Error),
// }

// impl std::fmt::Debug for LoginError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         error_chain_fmt(self, f)
//     }
// }

// impl IntoResponse for LoginError {
//     fn into_response(self) -> Response {
//         // match self {
//         //     Self::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
//         //     Self::UnexpectedError(_) => {
//         //         (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
//         //     }
//         // }

//         let query_string = format!("error={}", urlencoding::encode(&self.to_string()));
//         let secret: &[u8] = &[0; 32];
//         let hmac_tag = {
//             let mut mac = Hmac::<sha3::Sha3_256>::new_from_slice(secret).unwrap();
//             mac.update(query_string.as_bytes());
//             mac.finalize().into_bytes()
//         };

//         (
//             StatusCode::SEE_OTHER,
//             [(
//                 "Location",
//                 format!("/login?{query_string}&tag={hmac_tag:x}"),
//             )],
//         )
//             .into_response()
//     }
// }

// impl From<AuthError> for LoginError {
//     fn from(value: AuthError) -> Self {
//         match value {
//             AuthError::InvalidCredentials(_) => Self::AuthError(value.into()),
//             AuthError::UnexpectedError(_) => Self::UnexpectedError(value.into()),
//         }
//     }
// }
