use axum::{extract::rejection::JsonRejection, response::IntoResponse, Json};
use hyper::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[instrument(name = "Dummy implementation of newsletter", skip(body))]
pub async fn publish_newsletter(body: Result<Json<BodyData>, JsonRejection>) -> impl IntoResponse {
    let _body = match body {
        Ok(Json(body)) => body,
        Err(e) => {
            tracing::info!("Failed to extract json body: {}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    StatusCode::OK
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}
