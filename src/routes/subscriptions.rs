use axum::{extract, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::data::Data;

#[derive(serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

// #[axum::debug_handler]
pub async fn subscribe(
    extract::State(db_pool): extract::State<Data<sqlx::MySqlPool>>,
    form: extract::Form<FormData>,
) -> impl IntoResponse {
    match sqlx::query!(
        r#"
  INSERT INTO subscriptions (id, email, name, subscribed_at)
  VALUES (?, ?, ?, ?)
  "#,
        Uuid::new_v4().to_string(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            eprintln!("Failed to execute query: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
