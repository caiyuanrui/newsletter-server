use std::{future::IntoFuture, time::Duration};

use anyhow::Context;
use axum::{
    routing::{get, post},
    Router,
};
use axum_messages::MessagesManagerLayer;
use secrecy::ExposeSecret;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use tower::ServiceBuilder;
use tower_cookies::cookie::time;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    fred::prelude::{ClientLike, Config, Pool},
    RedisStore,
};

use crate::{
    appstate::{AppState, HmacSecret},
    authentication::reject_anonymous_user,
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        admin_dashboard, change_password, change_password_form, log_out, login, not_found,
        publish_newsletter, publish_newsletter_form,
    },
    routes::{confirm, health_check, home, login_form, subscribe},
    utils::{Data, Server},
};

fn run(
    listener: tokio::net::TcpListener,
    db_pool: MySqlPool,
    redis_pool: Pool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: HmacSecret,
) -> Server {
    let secret_key = tower_sessions::cookie::Key::from(hmac_secret.expose_secret().as_bytes());

    let base_url = super::appstate::ApplicationBaseUrl(base_url);
    let shared_state = AppState {
        db_pool,
        email_client: Data::new(email_client),
        base_url,
        hmac_secret,
    };

    let session_layer = SessionManagerLayer::new(RedisStore::new(redis_pool))
        .with_secure(false)
        .with_signed(secret_key)
        .with_expiry(Expiry::OnInactivity(time::Duration::minutes(10)));

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/", get(home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .nest(
            "/admin",
            Router::new()
                .route("/dashboard", get(admin_dashboard))
                .route("/password", get(change_password_form))
                .route("/password", post(change_password))
                .route("/logout", post(log_out))
                .route("/newsletters", get(publish_newsletter_form))
                .route("/newsletters", post(publish_newsletter))
                .route_layer(axum::middleware::from_fn(reject_anonymous_user)),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(session_layer)
                .layer(MessagesManagerLayer),
        )
        .fallback(not_found)
        .with_state(shared_state);

    Server::new(axum::serve(listener, app).into_future())
}

pub struct Application {
    port: u16,
    redis_pool: Pool,
    db_pool: MySqlPool,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let db_pool = get_connection_pool(&configuration.database);
        Self::build_with_db(configuration, db_pool).await
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

pub fn get_connection_pool(config: &DatabaseSettings) -> MySqlPool {
    MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}
