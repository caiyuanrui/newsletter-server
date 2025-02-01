use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use hyper::{header, StatusCode};
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{domain::SubscriberId, session_state::TypedSession, utils::e500};

#[instrument(name = "Admin Dashboard", skip(session))]
pub async fn admin_dashboard(
    session: TypedSession,
    State(db_pool): State<MySqlPool>,
) -> Result<Response, Response> {
    let username = if let Some(user_id) = session.get_user_id().await.map_err(e500)? {
        get_username(user_id, &db_pool).await.map_err(e500)?
    } else {
        return Ok((StatusCode::SEE_OTHER, [(header::LOCATION, "/login")]).into_response());
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
  <p>Available actions:</p>
  <ol>
    <li><a href="/admin/password">Change password</a></li>
  </ol>
</body>
</html>"#
    ))
    .into_response())
}

pub async fn get_username(
    user_id: SubscriberId,
    pool: &MySqlPool,
) -> Result<String, anyhow::Error> {
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
