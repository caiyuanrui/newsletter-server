use axum::{extract, routing::get, Router};

async fn greeter(path: Option<extract::Path<String>>) -> String {
    format!("Hello {}!", path.map(|e| e.0).unwrap_or("World".into()))
}

type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(greeter))
        .route("/{name}", get(greeter));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
