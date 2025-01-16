use axum::{http::StatusCode, response::IntoResponse};
use tracing::instrument;

#[instrument(name = "health check")]
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
