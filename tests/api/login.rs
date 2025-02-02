use sqlx::MySqlPool;

use crate::helpers::{assert_is_redirect_to, spawn_test_app};

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let login_body = serde_json::json!({"username": "random user", "password": "random password"});
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");

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

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}

#[sqlx::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
