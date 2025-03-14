use sqlx::MySqlPool;

use super::helpers::*;

#[sqlx::test]
async fn health_check_works(pool: MySqlPool) {
    // Arrage
    let test_app = spawn_test_app(pool).await;
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
