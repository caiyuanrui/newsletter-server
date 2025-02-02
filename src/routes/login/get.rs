use axum::response::{Html, IntoResponse};
use axum_messages::Messages;
use hyper::StatusCode;

use tracing::instrument;

#[instrument(name = "Get Login Form")]
pub async fn login_form(messages: Messages) -> impl IntoResponse {
    let error_html: String = messages
        .into_iter()
        .map(|message| format!("<p><i>{}</i></p>", message.message))
        .next()
        .unwrap_or_default();

    (
        StatusCode::OK,
        Html::from(format!(
            r#"<!doctype html>
    <html lang="en">
      <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8" />
        <title>Login</title>
      </head>
      <body>
      {error_html}
        <form action="/login" method="post">
          <label
            >Username
            <input
              type="text"
              placeholder="Enter Username"
              name="username"
              required
            /> </label
          ><label
            >Password
            <input
              type="password"
              placeholder="Enter Password"
              name="password"
              required
            />
          </label>
          <button type="submit">Login</button>
        </form>
      </body>
    </html>
"#,
        )),
    )
}
