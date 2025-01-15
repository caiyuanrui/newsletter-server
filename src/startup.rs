use axum::{
    routing::{get, post},
    serve::Serve,
    Router,
};
use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};

fn app() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
}

pub fn run(listener: TcpListener) -> Serve<TcpListener, Router, Router> {
    let app = app();
    axum::serve(listener, app)
}
