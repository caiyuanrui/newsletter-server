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
pub struct TestAPP {
    pub address: String,
    pub db_pool: sqlx::MySqlPool,
    pub email_server: MockServer,
}

impl TestAPP {
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
}

pub async fn spawn_app() -> TestAPP {
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

    let app = Application::build(&configuration)
        .await
        .expect("Failed to build the test application");
    let address = format!("http://127.0.0.1:{}", app.port());
    let db_pool = app.db_pool();

    tokio::spawn(app.run());

    TestAPP {
        address,
        db_pool,
        email_server,
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
