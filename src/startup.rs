use std::{future::IntoFuture, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::Request,
    http,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};

use http_body_util::BodyExt;

use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

use crate::configuration::DatabaseSettings;

use super::{
    configuration::Settings,
    email_client::EmailClient,
    routes::{health_check, subscribe},
    utils::{Data, Server},
};

pub struct Application {
    port: u16,
    db_pool: MySqlPool,
    server: Server,
}

impl Application {
    pub async fn build(configuration: &Settings) -> Result<Self, std::io::Error> {
        // database
        let db_pool = get_connection_pool(&configuration.database);
        // tcp lst
        let addr = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let port = listener.local_addr().unwrap().port();
        // email client
        let url = configuration
            .email_client
            .base_url
            .as_str()
            .try_into()
            .expect("Failed to parse the url");
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Failed to parse the email sender's name");
        let timeout = configuration.email_client.timeout();
        let authorization_token = configuration.email_client.authorization_token.clone();
        let email_client = EmailClient::new(url, sender_email, authorization_token, timeout);

        let server = run(listener, db_pool.clone(), email_client);

        Ok(Self {
            server,
            port,
            db_pool,
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn db_pool(&self) -> MySqlPool {
        self.db_pool.clone()
    }
}

fn get_connection_pool(config: &DatabaseSettings) -> MySqlPool {
    MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}

fn run(listener: tokio::net::TcpListener, db_pool: sqlx::MySqlPool, client: EmailClient) -> Server {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .layer(middleware::from_fn(print_request_response))
        .with_state(Data::new(db_pool))
        .with_state(Data::new(client));

    Server::new(axum::serve(listener, app).into_future())
}

async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (http::StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (http::StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                http::StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ))
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{direction} body = {body:?}");
    }

    Ok(bytes)
}
