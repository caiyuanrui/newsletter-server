use sqlx::MySqlPool;
use uuid::Uuid;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers::{spawn_test_app, ConfirmationLink, TestApp};

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(pool: MySqlPool) {
    let test_app = spawn_test_app(pool).await;
    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
      "title": "Newsletter Title",
      "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as html</p>"
      }
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(200, response.status().as_u16());
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(pool: MySqlPool) {
    let test_app = spawn_test_app(pool).await;
    create_confirmed_subscriber(&test_app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
      "title": "Newsletter Title",
      "content": {
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as html</p>"
      }
    });

    let response = test_app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(200, response.status().as_u16());
}

#[sqlx::test]
async fn newsletters_returns_400_for_invalid_data(pool: MySqlPool) {
    let test_app = spawn_test_app(pool).await;
    let test_cases = [
        (
            serde_json::json!({
              "content": {
                "html": "<p>Newsletter body as HTML</p>",
                "text": "Newsletter body as plain text"
              }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
              "title": "Newsletter!"
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_newsletters(&invalid_body).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the playload was {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn requests_missing_authorization_are_rejected(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&serde_json::json!({
          "title": "Newsletter title",
          "content": {
            "html": "<p>Newsletter body as HTML</p>",
            "text": "Newsletter body as plain text"
          }
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    )
}

#[sqlx::test]
async fn non_existing_user_is_rejected(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
          "title": "Newsletter title",
          "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
          }
        }))
        .send()
        .await
        .expect("Failed to execute response");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[sqlx::test]
async fn invalid_password_is_rejected(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let invalid_password = Uuid::new_v4().to_string();

    let request = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .basic_auth(app.test_user.username, Some(invalid_password))
        .json(&serde_json::json!({
          "title": "Newsletter title",
          "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
          }
        }))
        .send()
        .await
        .expect("Faild to execute response");

    assert_eq!(401, request.status().as_u16());
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLink {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;
    test_app.post_subscriptions(body).await;

    let email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    test_app.get_confirmation_links(&email_request).await
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(test_app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
