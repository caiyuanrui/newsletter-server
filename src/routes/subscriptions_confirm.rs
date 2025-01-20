use axum::{
    extract::{rejection::QueryRejection, Query},
    response::IntoResponse,
};
use hyper::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[instrument(name = "Confirm a pending subscriber")]
pub async fn confirm(params: Result<Query<Params>, QueryRejection>) -> impl IntoResponse {
    match params {
        Ok(Query(params)) => {
            tracing::info!(params.token);
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("{e}");
            StatusCode::BAD_REQUEST
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub token: String,
}
