use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use hyper::StatusCode;

pub async fn login() -> impl IntoResponse {
    Response::builder()
        .header("Location", "/")
        .status(StatusCode::SEE_OTHER)
        .body(Body::empty())
        .unwrap()
}
