use std::{future::IntoFuture, time::Duration};

use axum::{
    body::Body,
    http::Request,
    routing::{get, post},
    Router,
};

use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse};
use tracing::Level;

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

    pub fn db_pool(&self) -> &MySqlPool {
        &self.db_pool
    }
}

fn get_connection_pool(config: &DatabaseSettings) -> MySqlPool {
    MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}

pub fn run(
    listener: tokio::net::TcpListener,
    db_pool: sqlx::MySqlPool,
    client: EmailClient,
) -> Server {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(default_span)
                .on_request(DefaultOnRequest::new())
                .on_response(DefaultOnResponse::new())
                .on_failure(DefaultOnFailure::new()),
        )
        .with_state(Data::new(db_pool))
        .with_state(Data::new(client));

    Server::new(axum::serve(listener, app).into_future())
}

fn default_span(request: &Request<Body>) -> tracing::Span {
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
