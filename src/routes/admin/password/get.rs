use axum::response::{Html, IntoResponse, Response};

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password_form(session: TypedSession) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(_user_id) => Ok(Html(include_str!("password.html")).into_response()),
    }
}
