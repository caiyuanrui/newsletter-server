use axum::{
    routing::{get, post},
    serve::Serve,
    Router,
};
use tokio::net::TcpListener;

use crate::{
    data::Data,
    routes::{health_check, subscribe},
};

fn app(db_pool: Data<sqlx::MySqlPool>) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(db_pool)
}

pub fn run(listener: TcpListener, db_pool: sqlx::MySqlPool) -> Serve<TcpListener, Router, Router> {
    let app = app(Data::new(db_pool));
    axum::serve(listener, app)
}
