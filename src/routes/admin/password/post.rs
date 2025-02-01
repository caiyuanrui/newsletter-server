use axum::{response::Response, Form};
use secrecy::SecretString;
use uuid::Uuid;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password(
    session: TypedSession,
    Form(_form): Form<FormData>,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(_) => todo!(),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

impl Default for FormData {
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
