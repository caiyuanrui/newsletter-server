use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_messages::Messages;
use hyper::{header, StatusCode};
use tracing::instrument;

use crate::{session_state::TypedSession, utils::e500};

#[instrument(name = "reject anonymous user", skip_all)]
pub async fn reject_anonymous_user(
    session: TypedSession,
    // following layers and routes can't extract messages if we extract it here,
    // to reinsert this extension into req didn't work though, and idkw
    // messages: Messages,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(user_id);
            Ok(next.run(req).await)
        }
        None => {
            let messages = req
                .extensions()
                .get::<Messages>()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR.into_response())?
                .clone();
            messages.error("The user has not logged in.");
            Ok((StatusCode::SEE_OTHER, [(header::LOCATION, "/login")]).into_response())
        }
    }
}
