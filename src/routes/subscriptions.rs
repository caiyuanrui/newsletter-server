use axum::{extract, http::StatusCode, response::IntoResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

// #[axum::debug_handler]
pub async fn subscribe(_form: extract::Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}
