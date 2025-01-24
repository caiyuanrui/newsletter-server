use std::sync::LazyLock;

use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::Application,
    telementry::{get_subscriber, init_subscriber},
};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "debug".to_string();
    let subscriber_name = "test".to_string();
    match std::env::var("TEST_LOG") {
        Ok(_) => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
            init_subscriber(subscriber);
        }
        Err(_) => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
            init_subscriber(subscriber);
        }
    }
});

#[derive(Debug)]
pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::MySqlPool,
    pub email_server: MockServer,
    pub port: u16,
}

pub struct ConfirmationLink {
    pub html: reqwest::Url,
    #[allow(dead_code)]
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(
        &self,
        body: impl Into<reqwest::Body>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
    }

    pub async fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLink {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(1, links.len());
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!("127.0.0.1", confirmation_link.host_str().unwrap());
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLink { html, plain_text }
    }
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = uuid::Uuid::new_v4().to_string().replace("-", "");
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let app = Application::build(configuration)
        .await
        .expect("Failed to build the test application");
    let port = app.port();
    let address = format!("http://127.0.0.1:{}", app.port());
    let db_pool = app.db_pool();

    tokio::spawn(app.run());

    TestApp {
        address,
        db_pool,
        email_server,
        port,
    }
}

async fn configure_database(config: &DatabaseSettings) {
    use sqlx::{Connection, Executor};

    // create database
    let mut connection = sqlx::MySqlConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to MySQL.");
    connection
        .execute(format!(r#"CREATE DATABASE `{}`;"#, config.database_name).as_str())
        .await
        .unwrap_or_else(|_| panic!("Failed to create test database {}", config.database_name));

    // migrate databse
    let mut connection = sqlx::MySqlConnection::connect_with(&config.with_db())
        .await
        .expect("Failed to connect to MySQL.");
    sqlx::migrate!("./migrations")
        .run(&mut connection)
        .await
        .expect("Failed to migrate the databse.");
}
