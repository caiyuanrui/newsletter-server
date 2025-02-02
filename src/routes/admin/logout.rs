use axum::response::Response;
use axum_messages::Messages;
use tracing::instrument;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[instrument(name = "Logout", skip_all, fields(user_id))]
pub async fn log_out(session: TypedSession, messages: Messages) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(user_id) => {
            tracing::Span::current().record("user_id", user_id.to_string());

            _ = session.logout().await;
            messages.info("You have successfully logged out.");
            Ok(see_other("/login"))
        }
    }
}
