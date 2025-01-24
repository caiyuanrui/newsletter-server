use axum::response::IntoResponse;
use hyper::StatusCode;
use tracing::instrument;

#[instrument(name = "Dummy implementation of newsletter")]
pub async fn publish_newsletter() -> impl IntoResponse {
    StatusCode::OK
}
