use axum::response::{Html, IntoResponse};
use axum_messages::Messages;
use hyper::StatusCode;

use std::fmt::Write;

pub async fn publish_newsletter_form(messages: Messages) -> impl IntoResponse {
    let msg = messages.into_iter().fold(String::new(), |mut acc, item| {
        _ = writeln!(acc, "<p><i>{}</i></p>", item.message);
        acc
    });

    (
        StatusCode::OK,
        Html(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta http-equiv="content-type" content="text/html; charset=utf-8">
<title>Send Newsletter Issue</title>
</head>
<body>
  {msg}
  <form action="/admin/newsletters" method="post">
    <label>Newsletter Title
    <br />
      <textarea name="title" rows="10" cols="50"></textarea>
    </label>
    <br />
    <label>Newsletter Content in HTML Format
    <br />
      <textarea name="html_content" rows="20" cols="100"></textarea>
    </label>
    <br />
    <label>Newsletter Content in Plain Text
    <br />
      <textarea name="text_content" rows="20" cols="100"></textarea>
    </label>
    <br />
    <button type="submit">Send</button>
  </form>
</body>
</html>"#,
        )),
    )
}
