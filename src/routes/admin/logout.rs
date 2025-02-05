use axum::response::Response;
use axum_messages::Messages;
use tracing::instrument;

use crate::{session_state::TypedSession, utils::see_other};

#[instrument(name = "Logout", skip_all)]
pub async fn log_out(session: TypedSession, messages: Messages) -> Result<Response, Response> {
    if session.logout().await.is_err() {
        tracing::error!("Failed to log out this user's session");
    }
    messages.info("You have successfully logged out.");
    Ok(see_other("/login"))
}
