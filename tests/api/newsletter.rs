use serde::Serialize;
use sqlx::MySqlPool;
use wiremock::{matchers, Mock, ResponseTemplate};

use crate::helpers::{assert_is_redirect_to, spawn_test_app, ConfirmationLink, TestApp};

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_send_newsletters_form(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let response = app.get_newsletters().await;
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn you_must_be_logged_in_to_send_newsletters(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let newsletter_request_body = serde_urlencoded::to_string([
        ("title", "Newsletter Title"),
        ("html_content", "<p>Newsletter body as html</p>"),
        ("text_content", "Newsletter body as plain text"),
    ])
    .unwrap();

    let response = app.post_newsletters(newsletter_request_body).await;
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

    let newsletter_request_body = serde_urlencoded::to_string([
        ("title", "Newsletter Title"),
        ("html_content", "<p>Newsletter body as html</p>"),
        ("text_content", "Newsletter body as plain text"),
    ])
    .unwrap();

    let response = app.post_newsletters(newsletter_request_body).await;

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

    let newsletter_request_body = serde_urlencoded::to_string([
        ("title", "Newsletter Title"),
        ("html_content", "<p>Newsletter body as html</p>"),
        ("text_content", "Newsletter body as plain text"),
    ])
    .unwrap();

    let response = app.post_newsletters(newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");
}

#[sqlx::test]
async fn newsletters_returns_422_for_invalid_data(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let test_cases = [
        (
            serde_urlencoded::to_string([
                ("html_content", "<p>Newsletter body as html</p>"),
                ("text_content", "Newsletter body as plain text"),
            ])
            .unwrap(),
            "missing title",
        ),
        (
            serde_urlencoded::to_string([("title", "Newsletter Title")]).unwrap(),
            "missing content",
        ),
    ];

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(invalid_body).await;

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

    let form = NewsletterFormBuilder::new()
        .with_title("Newsletter title")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .to_string();

    let response = app.post_newsletters(form).await;
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

    let form = NewsletterFormBuilder::new()
        .with_title("Newsletter title")
        .with_html_content("<p>Newsletter body as HTML</p>")
        .with_text_content("Newsletter body as plain text")
        .to_string();

    let response = app.post_newsletters(form).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_newsletters_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"))
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

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[derive(Debug, Serialize)]
struct NewsletterFormBuilder<S>
where
    S: Serialize,
{
    title: Option<S>,
    html_content: Option<S>,
    text_content: Option<S>,
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
