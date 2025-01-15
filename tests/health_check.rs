use std::future::IntoFuture;

use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
};

#[derive(Debug)]
pub struct TestAPP {
    pub address: String,
    pub db_pool: sqlx::MySqlPool,
}

async fn spawn_app() -> TestAPP {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port.");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string().replace("-", "");
    let db_pool = configure_database(&configuration.database).await;

    let serve = run(listener, db_pool.clone());
    tokio::spawn(serve.into_future());

    TestAPP { address, db_pool }
}

pub async fn configure_database(config: &DatabaseSettings) -> sqlx::MySqlPool {
    use sqlx::{Connection, Executor};

    // create database
    let mut connection = sqlx::MySqlConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to MySQL.");
    connection
        .execute(format!(r#"CREATE DATABASE {};"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // migrate databse
    let db_pool = sqlx::MySqlPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to MySQL.");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the databse.");

    db_pool
}

#[tokio::test]
async fn health_check_works() {
    // Arrage
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &test_app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_404_when_data_is_missing() {
    // Arrange
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    // Act
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscription", &test_app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            404,
            response.status().as_u16(),
            "The API did not return a 404 when the payload was {}.",
            error_message
        );
    }
}
