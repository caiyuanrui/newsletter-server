use core::fmt;

use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use hyper::StatusCode;
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{appstate::AppState, domain::SubscriberId, session_state::TypedSession};

#[instrument(name = "Admin Dashboard", skip(session, app_state))]
pub async fn admin_dashboard(
    session: TypedSession,
    State(app_state): State<AppState>,
) -> Result<Response, Response> {
    let username = if let Some(user_id) = session.get_user_id().await.map_err(e500)? {
        get_username(user_id, &app_state.db_pool)
            .await
            .map_err(e500)?
    } else {
        todo!()
    };

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta http-equiv="content-type" content="text/html; charset=utf-8">
  <title>Admin dashboard</title>
</head>
<body>
  <p>Welcome {username}!</p>
</body>
</html>"#
    ))
    .into_response())
}

async fn get_username(user_id: SubscriberId, pool: &MySqlPool) -> Result<String, anyhow::Error> {
    sqlx::query!(
        r#"
      SELECT username
      FROM users
      WHERE user_id = ?
      "#,
        user_id.to_string()
    )
    .fetch_one(pool)
    .await
    .map(|row| row.username)
    .context("Failed to perform a query to retrieve a username")
}

fn e500<T>(e: T) -> Response
where
    T: fmt::Debug + fmt::Display + 'static,
{
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}
