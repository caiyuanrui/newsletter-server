use wiremock::{matchers, Mock, ResponseTemplate};

use super::helpers::*;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let test_app = spawn_app().await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app
        .post_subscriptions(body)
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subscriptions(body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation")
}

#[tokio::test]
async fn subscribe_returns_a_404_when_data_is_missing() {
    // Arrange
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    // Act
    for (body, message) in test_cases {
        let response = test_app
            .post_subscriptions(body)
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            404,
            response.status().as_u16(),
            "The API did not return a 404 when the payload was {}.",
            message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=le%20guin&email=", "empty email"),
        ("name=le%20guin&email=ursula_le_guin%40", "invalid email"),
    ];

    // Act
    for (body, message) in test_cases {
        let response = test_app
            .post_subscriptions(body)
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 when the payload was {}.",
            message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subscriptions(body).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let _res = test_app.post_subscriptions(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_links = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(1, links.len());
        links[0].as_str().to_owned()
    };

    let html_link = get_links(body["HtmlBody"].as_str().unwrap());
    let text_link = get_links(body["TextBody"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token")
        .execute(&test_app.db_pool)
        .await
        .expect("Failed to drop subscription_token field");

    let response = test_app.post_subscriptions(body).await.unwrap();

    assert_eq!(500, response.status().as_u16());
}
