use axum::response::{Html, IntoResponse, Response};
use hyper::StatusCode;
use tokio::fs;
use tracing::instrument;

#[instrument(name = "Home")]
pub async fn home() -> Response {
    match fs::read_to_string("public/index.html").await {
        Ok(html_content) => (StatusCode::OK, Html::from(html_content)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
