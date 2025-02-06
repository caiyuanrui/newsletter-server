use axum::response::IntoResponse;
use hyper::StatusCode;
use tracing::instrument;

#[instrument(name = "Not Found")]
pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404: Page Not Found")
}
