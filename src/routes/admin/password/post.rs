use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form,
};
use axum_messages::Messages;
use hyper::{header, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::ser::SerializeStruct;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::admin::dashboard::get_username,
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password(
    session: TypedSession,
    State(pool): State<MySqlPool>,
    messages: Messages,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(user_id) => {
            if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
                messages.error(
                    "You entered two different new passwords - the field values must match.",
                );

                return Ok((
                    StatusCode::SEE_OTHER,
                    [(header::LOCATION, "/admin/password")],
                )
                    .into_response());
            }

            let username = get_username(user_id, &pool).await.map_err(e500)?;

            let credentials = Credentials {
                username,
                password: form.current_password,
            };

            match validate_credentials(credentials, &pool).await {
                Ok(_) => todo!(),
                Err(AuthError::InvalidCredentials(_)) => {
                    messages.error("The current password is incorrect.");
                    Ok(see_other("/admin/password"))
                }
                Err(e @ AuthError::UnexpectedError(_)) => Err(e500(e)),
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
