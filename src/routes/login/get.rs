use axum::{
    extract::Query,
    response::{Html, IntoResponse},
};
use hyper::StatusCode;
use serde::Deserialize;

pub async fn login_form(Query(query): Query<QueryParams>) -> impl IntoResponse {
    let error_html = query
        .error
        .map(|c| format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&c)))
        .unwrap_or("".into());
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
"#
        )),
    )
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub error: Option<String>,
}
