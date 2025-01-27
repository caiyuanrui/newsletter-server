use axum::response::{Html, IntoResponse};
use hyper::StatusCode;

pub async fn login_form() -> impl IntoResponse {
    match tokio::fs::read_to_string("public/index.html").await {
        Ok(html_content) => (StatusCode::OK, Html::from(html_content)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
