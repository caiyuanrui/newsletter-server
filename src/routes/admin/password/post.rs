use axum::{
    extract::{Extension, State},
    response::Response,
    Form,
};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use serde::ser::SerializeStruct;
use sqlx::MySqlPool;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    authentication::{self, validate_credentials, AuthError, Credentials},
    domain::UserId,
    routes::admin::dashboard::get_username,
    utils::{e500, see_other},
};

#[instrument(name = "change password", skip_all)]
pub async fn change_password(
    State(pool): State<MySqlPool>,
    messages: Messages,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    // Reject: Two different new passwords
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        messages.error("You entered two different new passwords - the field values must match.");
        return Ok(see_other("/admin/password"));
    }

    // Reject: The new password is too simple or too long
    let password_length = form.new_password.expose_secret().len();
    if password_length <= 12 {
        messages.error("The new password is too short.");
        return Ok(see_other("/admin/password"));
    }
    if password_length >= 128 {
        messages.error("The new password is too long.");
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            authentication::change_password(user_id, form.new_password, &pool)
                .await
                .map_err(e500)?;
            messages.error("Your password has been changed.");
            Ok(see_other("/admin/password"))
        }
        Err(AuthError::InvalidCredentials(_)) => {
            messages.error("The current password is incorrect.");
            Ok(see_other("/admin/password"))
        }
        Err(e @ AuthError::UnexpectedError(_)) => Err(e500(e)),
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
