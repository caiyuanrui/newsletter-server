use std::future::Future;

use axum::{
    body::Body,
    http::Request,
    routing::{get, post},
    Router,
};
use axum_server::Server;
use std::net::TcpListener;
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse};
use tracing::Level;

use crate::{
    data::Data,
    email_client::EmailClient,
    routes::{health_check, subscribe},
};

fn app(db_pool: Data<sqlx::MySqlPool>, client: EmailClient) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(make_default_span())
                .on_request(DefaultOnRequest::new())
                .on_response(DefaultOnResponse::new())
                .on_failure(DefaultOnFailure::new()),
        )
        .with_state(db_pool)
        .with_state(Data::new(client))
}

pub fn run(
    listener: TcpListener,
    db_pool: sqlx::MySqlPool,
    client: EmailClient,
) -> impl Future<Output = std::io::Result<()>> {
    let app = app(Data::new(db_pool), client);
    Server::from_tcp(listener).serve(app.into_make_service())
}

fn make_default_span() -> impl Fn(&Request<Body>) -> tracing::Span + Clone {
    |request: &Request<Body>| {
        let request_id = uuid::Uuid::new_v4();
        tracing::span!(
            Level::DEBUG,
            "request",
            method = tracing::field::display(request.method()),
            uri = tracing::field::display(request.uri()),
            version = tracing::field::debug(request.version()),
            request_id = tracing::field::display(request_id),
        )
    }
}
