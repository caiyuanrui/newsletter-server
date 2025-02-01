use secrecy::ExposeSecret;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, generate_change_password_form, spawn_test_app};

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_change_password_form(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let response = app.get_change_password().await;
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn you_must_be_logged_in_to_change_your_password(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;

    let response = app
        .post_change_password(&generate_change_password_form())
        .await;
    assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn new_password_fields_must_match(pool: MySqlPool) {
    let app = spawn_test_app(pool).await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    let response = app
        .post_change_password(&serde_json::json!({
          "current_password": &app.test_user.password,
          "new_password": &new_password,
          "new_password_check": &another_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[sqlx::test]
async fn current_password_must_be_valid(pool: MySqlPool) {
    use zero2prod::routes::PasswordFormData;
    let app = spawn_test_app(pool).await;

    // Login
    app.post_login(&serde_json::json!({
      "username": app.test_user.username,
      "password": app.test_user.password
    }))
    .await;

    // Try to change password
    let form_data = PasswordFormData::default();
    assert_ne!(
        form_data.current_password.expose_secret(),
        app.test_user.password,
        "We should input a wrong password"
    );
    let ser = serde_json::to_value(form_data).unwrap();
    let response = app.post_change_password(&ser).await;

    assert_is_redirect_to(&response, "/admin/password");

    // Follow the redirection
    let html_page = app.get_admin_dashboard_html().await;
    println!("{}", html_page);
    assert!(html_page.contains("The current password is incorrect."));
}
