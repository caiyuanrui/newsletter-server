use axum::{extract, http::StatusCode, response::IntoResponse, routing::get, Router};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn greeter(path: Option<extract::Path<String>>) -> String {
    format!("Hello {}!", path.map(|e| e.0).unwrap_or("World".into()))
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(greeter))
        .route("/{name}", get(greeter))
        .route("/health_check", get(health_check));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
