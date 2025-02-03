use axum::{response::Response, Extension};
use axum_messages::Messages;
use tracing::instrument;

use crate::{domain::UserId, session_state::TypedSession, utils::see_other};

#[instrument(name = "Logout", skip_all, fields(user_id))]
pub async fn log_out(
    session: TypedSession,
    messages: Messages,
    Extension(user_id): Extension<UserId>,
) -> Result<Response, Response> {
    tracing::Span::current().record("user_id", user_id.to_string());
    _ = session.logout().await;
    messages.info("You have successfully logged out.");
    Ok(see_other("/login"))
}
