use std::sync::LazyLock;

use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    email_client::EmailClient,
    startup::run,
    startup::Application,
    telementry::{get_subscriber, init_subscriber},
};

#[derive(Debug)]
pub struct TestAPP {
    pub address: String,
    pub db_pool: sqlx::MySqlPool,
}

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

pub async fn spawn_app() -> TestAPP {
    LazyLock::force(&TRACING);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port.");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string().replace("-", "");
    configuration.application.port = 0;

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Failed to parse the email name of the sender");
    let url = configuration
        .email_client
        .base_url
        .as_str()
        .try_into()
        .unwrap();
    let timeout = configuration.email_client.timeout();
    let authorization_token = configuration.email_client.authorization_token;
    let email_client = EmailClient::new(url, sender_email, authorization_token, timeout);

    let db_pool = configure_database(&configuration.database).await;

    let server = run(listener, db_pool.clone(), email_client);
    tokio::spawn(server);

    // let app = Application::build(&configuration).await.unwrap();
    // let address = format!("http://localhost:{}", app.port());
    // let db_pool = app.db_pool().clone();
    // tokio::spawn(app.run());

    TestAPP { address, db_pool }
}

pub async fn configure_database(config: &DatabaseSettings) -> sqlx::MySqlPool {
    use sqlx::{Connection, Executor};

    // create database
    let mut connection = sqlx::MySqlConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to MySQL.");
    connection
        .execute(format!(r#"CREATE DATABASE `{}`;"#, config.database_name).as_str())
        .await
        .unwrap_or_else(|_| panic!("Failed to create database {}.", config.database_name));

    // migrate databse
    let db_pool = sqlx::MySqlPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to MySQL.");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the databse.");

    db_pool
}
