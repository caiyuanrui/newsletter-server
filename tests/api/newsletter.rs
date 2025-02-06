use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers::{spawn_app, ConfirmationLink, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
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

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &test_app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .unwrap();

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
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

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", test_app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16())
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
    test_app.post_subscriptions(body).await.unwrap();

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
