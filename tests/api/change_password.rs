use sqlx::MySqlPool;

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
