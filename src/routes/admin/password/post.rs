use axum::{body::Body, extract::State, response::Response, Form};
use hyper::{header, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::ser::SerializeStruct;
use sqlx::MySqlPool;
use tower_cookies::Cookie;
use uuid::Uuid;

use crate::{
    appstate::HmacSecret,
    authentication::{validate_credentials, AuthError, Credentials},
    routes::{admin::dashboard::get_username, SignedCookieValue},
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password(
    session: TypedSession,
    State(pool): State<MySqlPool>,
    State(secret): State<HmacSecret>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(user_id) => {
            if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
                let cookie_value = SignedCookieValue::new(
                    "You entered two different new passwords - the field values must match.".into(),
                    &secret,
                );
                let cookie = Cookie::build(("_flash", cookie_value.into_json()))
                    .http_only(true)
                    .secure(true)
                    .build();
                let response = Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header(header::LOCATION, "/admin/password")
                    .header(header::SET_COOKIE, cookie.to_string())
                    .body(Body::empty())
                    .unwrap();
                return Ok(response);
            }

            let username = get_username(user_id, &pool).await.map_err(e500)?;

            let credentials = Credentials {
                username,
                password: form.current_password,
            };

            match validate_credentials(credentials, &pool).await {
                Ok(_) => todo!(),
                Err(AuthError::InvalidCredentials(_)) => {
                    let cookie_value =
                        SignedCookieValue::new("The current password is incorrect".into(), &secret);
                    let cookie = Cookie::build(("_flash", cookie_value.into_json()))
                        .http_only(true)
                        .secure(true)
                        .build();
                    Ok(Response::builder()
                        .status(StatusCode::SEE_OTHER)
                        .header(header::LOCATION, "/admin/password")
                        .header(header::SET_COOKIE, cookie.to_string())
                        .body(Body::empty())
                        .unwrap())
                }
                Err(AuthError::UnexpectedError(e)) => Err(e500(e)),
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    pub current_password: SecretString,
    pub new_password: SecretString,
    pub new_password_check: SecretString,
}

impl serde::Serialize for FormData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FormData", 3)?;
        state.serialize_field("current_password", self.current_password.expose_secret())?;
        state.serialize_field("new_password", self.new_password.expose_secret())?;
        state.serialize_field(
            "new_password_check",
            self.new_password_check.expose_secret(),
        )?;
        state.end()
    }
}

impl Default for FormData {
    /// Use this to generate a random form data.
    fn default() -> Self {
        let current_password = Uuid::new_v4().to_string();
        let new_password = Uuid::new_v4().to_string();
        let new_password_check = new_password.clone();

        Self {
            current_password: current_password.into(),
            new_password: new_password.into(),
            new_password_check: new_password_check.into(),
        }
    }
}
