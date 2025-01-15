use axum::routing::get;
use axum::{extract, http::StatusCode, response::IntoResponse, serve::Serve, Router};
use tokio::net::TcpListener;

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

async fn _greeter(path: Option<extract::Path<String>>) -> String {
    format!("Hello {}!", path.map(|e| e.0).unwrap_or("World".into()))
}

pub fn app() -> Router {
    Router::new().route("/health_check", get(health_check))
}

pub fn run(listener: TcpListener) -> Serve<TcpListener, Router, Router> {
    let app = app();
    axum::serve(listener, app)
}
