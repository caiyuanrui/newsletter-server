use std::future::Future;

use axum::{
    routing::{get, post},
    Router,
};
use axum_server::Server;
use tokio::net::TcpListener;
use tracing::Level;

use crate::{
    data::Data,
    routes::{health_check, subscribe},
};

fn app(db_pool: Data<sqlx::MySqlPool>) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .layer(
            tower_http::trace::TraceLayer::new_for_http().make_span_with(
                |request: &axum::http::Request<axum::body::Body>| {
                    let request_id = uuid::Uuid::new_v4();
                    tracing::span!(
                        Level::DEBUG,
                        "request",
                        method = tracing::field::display(request.method()),
                        uri = tracing::field::display(request.uri()),
                        version = tracing::field::debug(request.version()),
                        request_id = tracing::field::display(request_id),
                    )
                },
            ),
        )
        .with_state(db_pool)
}

pub fn run(
    listener: TcpListener,
    db_pool: sqlx::MySqlPool,
) -> impl Future<Output = std::io::Result<()>> {
    let app = app(Data::new(db_pool));
    Server::from_tcp(listener.into_std().unwrap()).serve(app.into_make_service())
}
