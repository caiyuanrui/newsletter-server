use fake::Fake;
use serde::Serialize;
use sqlx::MySqlPool;
use uuid::Uuid;
use wiremock::{matchers, Mock, MockBuilder, ResponseTemplate};

use std::time::Duration;

use crate::helpers::{
    assert_is_redirect_to, assert_publish_is_successful, spawn_test_app, ConfirmationLink, TestApp,
};

#[sqlx::test]
async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let newsletter_request_body = newsletter_request_body();
    // Two subscribers
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    // Email delivery fails for the second subscriber
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    // One of the subscriber delivery failed, the idempotency operation will roll back
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_eq!(500, response.status().as_u16());

    // Retry for submitting the form
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Delivery retry")
        .mount(&app.email_server)
        .await;
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_eq!(303, response.status().as_u16());
}

#[sqlx::test]
async fn concurrent_form_submission_is_handled_gracefully(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap();

    let response1 = app.post_publish_newsletters(&newsletter_request_body);
    let response2 = app.post_publish_newsletters(&newsletter_request_body);

    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

#[sqlx::test]
async fn newsletter_creation_is_idempotent(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Submit newsletter form
    let newsletter_request_body = NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap();
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletters_html().await;
    assert_publish_is_successful(&html_page);

    // Submit newsletter form again!
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
    let html_page = app.get_publish_newsletters_html().await;
    assert_publish_is_successful(&html_page);
}

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_send_newsletters_form(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let response = app.get_publish_newsletters().await;
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn you_must_be_logged_in_to_send_newsletters(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let newsletter_request_body = NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap();
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(matchers::any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let newsletter_request_body = NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap();
    let response = app.post_publish_newsletters(&newsletter_request_body).await;

    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[sqlx::test]
async fn newsletters_are_delivered_to_confirmed_subscribers(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    create_confirmed_subscriber(&app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let newsletter_request_body = NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap();
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[sqlx::test]
async fn newsletters_returns_422_for_invalid_data(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let test_cases = [
        (
            serde_json::json!({
            "html_content": "<p>Newsletter body as html</p>",
            "text_content": "Newsletter body as plain text",
            "idempotency_key": Uuid::new_v4().to_string()
            }),
            "missing title",
        ),
        (
            serde_json::json!({
              "title": "Newsletter Title",
              "idempotency_key": Uuid::new_v4().to_string()
            }),
            "missing content",
        ),
    ];

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    for (invalid_body, error_message) in test_cases {
        let response = app.post_publish_newsletters(&invalid_body).await;

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the playload was {}",
            error_message
        );
    }
}

#[sqlx::test]
async fn requests_missing_authorization_are_rejected(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
        .post(format!("{}/admin/newsletters", app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(
            NewsletterFormBuilder::new()
                .with_title("Newsletter title")
                .with_html_content("<p>Newsletter body as HTML</p>")
                .with_text_content("Newsletter body as plain text")
                .with_idempotency_key(Uuid::new_v4().to_string().as_str())
                .to_string(),
        )
        .send()
        .await
        .expect("Failed to execute request");
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn returns_422_if_published_newsletter_form_cannot_be_parsed(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let newsletter_request_body = serde_json::json!({
      "title": "Newsletter Title",
      "html_content": "<p>Newsletter body as html</p>",
      "invalid_field": "???"
    });
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_eq!(422, response.status().as_u16());
}

#[sqlx::test]
async fn succeed_to_publish_a_newsletter_form(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let newsletter_request_body = serde_json::json!({
      "title": "Newsletter Title",
      "html_content": "<p>Newsletter body as html</p>",
      "text_content": "Newsletter body as plain text",
      "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletters_html().await;
    assert_publish_is_successful(&html_page);
}

#[sqlx::test]
async fn create_10_confirmed_subscribers_and_send_a_newsletter_to_them(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let total_subscriber_nums = 10;
    for _ in 0..total_subscriber_nums {
        create_confirmed_subscriber(&app).await;
    }

    // Login
    let response = app
        .post_login(&serde_json::json!({
          "username": app.test_user.username,
          "password": app.test_user.password
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // Send a newsletter to all subscribers
    assert_eq!(
        send_a_newsletter_to_all_subscribers(&app).await,
        total_subscriber_nums
    );

    // We have a welcome email to every new subsriber.
    // And that's why we multiply `total_subscriber_nums` by 2.
    assert_eq!(
        total_subscriber_nums as usize * 2,
        app.email_server.received_requests().await.unwrap().len()
    );
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLink {
    let name: String = fake::faker::name::en::Name().fake();
    let email: String = fake::faker::internet::en::SafeEmail().fake();
    let body = serde_urlencoded::to_string([("name", name), ("email", email)]).unwrap();
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

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

async fn send_a_newsletter_to_all_subscribers(app: &TestApp) -> u64 {
    let total_subscriber_nums = sqlx::query!(r#"SELECT COUNT(*) as total FROM subscriptions"#)
        .fetch_one(&app.db_pool)
        .await
        .unwrap()
        .total as u64;
    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(total_subscriber_nums)
        .mount_as_scoped(&app.email_server)
        .await;
    let newsletter_request_body = serde_json::json!({
      "title": "Newsletter Title",
      "html_content": "<p>Newsletter body as html</p>",
      "text_content": "Newsletter body as plain text",
      "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletters(&newsletter_request_body).await;

    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletters_html().await;
    assert_publish_is_successful(&html_page);

    total_subscriber_nums
}

#[derive(Debug, Serialize)]
struct NewsletterFormBuilder<S>
where
    S: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<S>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<S>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_content: Option<S>,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<S>,
}

impl<S> NewsletterFormBuilder<S>
where
    S: Serialize,
{
    fn new() -> Self {
        Self {
            title: None,
            html_content: None,
            text_content: None,
            idempotency_key: None,
        }
    }

    fn with_title(self, title: S) -> Self {
        let mut this = self;
        this.title = Some(title);
        this
    }

    fn with_html_content(self, html_content: S) -> Self {
        let mut this = self;
        this.html_content = Some(html_content);
        this
    }

    fn with_text_content(self, text_content: S) -> Self {
        let mut this = self;
        this.text_content = Some(text_content);
        this
    }

    fn with_idempotency_key(self, idempotency_key: S) -> Self {
        let mut this = self;
        this.idempotency_key = Some(idempotency_key);
        this
    }

    fn into_json(self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl<S> core::fmt::Display for NewsletterFormBuilder<S>
where
    S: Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ser = serde_urlencoded::to_string(self).unwrap();
        ser.fmt(f)
    }
}

fn newsletter_request_body() -> serde_json::Value {
    NewsletterFormBuilder::new()
        .with_title("Newsletter Title")
        .with_text_content("Newsletter body as plain text")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_idempotency_key(Uuid::new_v4().to_string().as_str())
        .into_json()
        .unwrap()
}

/// Short-hand for a common mocking setup.
fn when_sending_an_email() -> MockBuilder {
    Mock::given(matchers::path("/email")).and(matchers::method("POST"))
}
