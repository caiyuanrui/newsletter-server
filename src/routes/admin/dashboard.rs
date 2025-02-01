use axum::response::IntoResponse;
use hyper::StatusCode;
use tracing::instrument;

#[instrument(name = "Admin Dashboard")]
pub async fn admin_dashboard() -> impl IntoResponse {
    StatusCode::OK
}
