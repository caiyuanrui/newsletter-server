use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    Extension,
};
use sqlx::MySqlPool;
use tracing::instrument;

use crate::{domain::UserId, utils::e500};

#[instrument(name = "Admin Dashboard", skip_all)]
pub async fn admin_dashboard(
    State(db_pool): State<MySqlPool>,
    Extension(user_id): Extension<UserId>,
) -> Result<Response, Response> {
    let username = get_username(user_id, &db_pool).await.map_err(e500)?;

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
    <li>
      <form name="logoutForm" action="/admin/logout" method="post">
        <input type="submit" value="Logout"/>
      </form>
    </li>
    <li>
      <a href="/admin/newsletters">Send a newsletter issue</a>
    </li>
  </ol>
</body>
</html>"#
    ))
    .into_response())
}

pub async fn get_username(user_id: UserId, pool: &MySqlPool) -> Result<String, anyhow::Error> {
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
