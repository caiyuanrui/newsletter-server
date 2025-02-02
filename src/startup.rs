use std::{future::IntoFuture, time::Duration};

use anyhow::Context;
use axum::{
    body::{Body, Bytes},
    extract::Request,
    http,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum_messages::{Messages, MessagesManagerLayer};
use http_body_util::BodyExt;
use secrecy::ExposeSecret;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use tower_cookies::{cookie::time, Key};
use tower_http::cors::CorsLayer;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    fred::prelude::{ClientLike, Config, Pool},
    RedisStore,
};
use tracing::Instrument;

use crate::{
    appstate::HmacSecret,
    routes::{admin_dashboard, change_password, change_password_form, login, not_found},
};

use super::{
    appstate::AppState,
    configuration::DatabaseSettings,
    configuration::Settings,
    email_client::EmailClient,
    routes::{confirm, health_check, home, login_form, publish_newsletter, subscribe},
    utils::{Data, Server},
};

fn run(
    listener: tokio::net::TcpListener,
    db_pool: sqlx::MySqlPool,
    redis_pool: Pool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: HmacSecret,
) -> Server {
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());

    let base_url = super::appstate::ApplicationBaseUrl(base_url);
    let shared_state = AppState {
        db_pool,
        email_client: Data::new(email_client),
        base_url,
        hmac_secret,
    };

    let redis_store = RedisStore::new(redis_pool);
    let session_layer = SessionManagerLayer::new(redis_store)
        .with_secure(false)
        .with_signed(secret_key)
        .with_expiry(Expiry::OnInactivity(time::Duration::seconds(10)));

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .route("/", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/admin/dashboard", get(admin_dashboard))
        .route("/admin/password", get(change_password_form))
        .route("/admin/password", post(change_password))
        .route("/read-messages", get(read_messages_handler))
        .route("/set-messages", get(set_messages_handler))
        .fallback(not_found)
        .layer(tower_cookies::CookieManagerLayer::new())
        .layer(CorsLayer::permissive())
        .layer(MessagesManagerLayer)
        .layer(session_layer)
        .layer(middleware::from_fn(print_request_response))
        .with_state(shared_state);

    Server::new(axum::serve(listener, app).into_future())
}

async fn set_messages_handler(messages: Messages) -> impl IntoResponse {
    messages
        .info("Hello, world!")
        .debug("This is a debug message.");

    axum::response::Redirect::to("/read-messages")
}

async fn read_messages_handler(messages: Messages) -> impl IntoResponse {
    let messages = messages
        .into_iter()
        .map(|message| format!("{}: {}", message.level, message))
        .collect::<Vec<_>>()
        .join(", ");

    if messages.is_empty() {
        "No messages yet!".to_string()
    } else {
        messages
    }
}

pub struct Application {
    port: u16,
    redis_pool: Pool,
    db_pool: MySqlPool,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        // redis
        let redis_config = Config::from_url(configuration.redis_uri.expose_secret())?;
        let redis_pool = Pool::new(redis_config, None, None, None, 6)?;

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

        let server = run(
            listener,
            db_pool.clone(),
            redis_pool.clone(),
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
        );

        Ok(Self {
            server,
            port,
            db_pool,
            redis_pool,
        })
    }

    /// This is used for test.
    pub async fn build_with_db(
        configuration: Settings,
        pool: MySqlPool,
    ) -> Result<Self, anyhow::Error> {
        // redis
        let redis_config = Config::from_url(configuration.redis_uri.expose_secret())?;
        let redis_pool = Pool::new(redis_config, None, None, None, 6)
            .context("Failed to create the redis pool")?;

        // database
        let db_pool = pool;
        // tcp lst
        let addr = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind tcp listener: {} is already in use", addr))?;
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

        let server = run(
            listener,
            db_pool.clone(),
            redis_pool.clone(),
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
        );

        Ok(Self {
            server,
            port,
            db_pool,
            redis_pool,
        })
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        let conn = self.redis_pool.connect();
        self.redis_pool.wait_for_connect().await?;
        self.server.await?;
        conn.await??;
        Ok(())
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

async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (http::StatusCode, String)> {
    let uri = req.uri();
    let method = req.method();

    let span = tracing::info_span!("http request=", method = %method, uri = %uri);

    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).instrument(span).await;

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
