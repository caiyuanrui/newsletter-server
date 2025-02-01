use sqlx::MySqlPool;
use zero2prod::routes::SignedCookieValue;

use crate::helpers::{assert_is_redirect_to, spawn_test_app};

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let login_body = serde_json::json!({"username": "random user", "password": "random password"});
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");

    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    let flash_cookie_value =
        SignedCookieValue::from_json(flash_cookie.value()).expect("Failed to parse cookie domain");
    assert_eq!("Authentication failed", flash_cookie_value.message);

    let html_page = app.get_login_form().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    let html_page = app.get_login_form().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
}

#[sqlx::test]
async fn redirect_to_admin_dashboard_after_login_success(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let login_body = serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
