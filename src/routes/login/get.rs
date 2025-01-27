use axum::response::{Html, IntoResponse};
use hyper::StatusCode;

pub async fn login_form() -> impl IntoResponse {
    (StatusCode::OK, Html::from(include_str!("login.html")))
}
