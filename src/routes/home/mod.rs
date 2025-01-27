use axum::response::{Html, IntoResponse};
use hyper::StatusCode;
use tracing::instrument;

#[instrument(name = "Home")]
pub async fn home() -> impl IntoResponse {
    (StatusCode::OK, Html::from(include_str!("home.html")))
}
