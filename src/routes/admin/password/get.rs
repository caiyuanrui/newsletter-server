use axum::response::{Html, IntoResponse, Response};
use axum_messages::Messages;
use hyper::StatusCode;
use tracing::instrument;

use std::fmt::Write;

use crate::{
    session_state::TypedSession,
    utils::{e500, see_other},
};

#[instrument(name = "Change password form", skip_all, fields(user_id, messages))]
pub async fn change_password_form(
    session: TypedSession,
    messages: Messages,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(user_id) => {
            let msg_html: String = messages.into_iter().fold(String::new(), |mut acc, item| {
                _ = writeln!(acc, "<p><i>{}</i></p>", item.message);
                acc
            });

            tracing::Span::current().record("user_id", user_id.to_string());
            tracing::Span::current().record("messages", &msg_html);

            Ok((
                StatusCode::OK,
                Html(format!(
                    r#"<!doctype html>
            <html lang="en">
              <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8" />
                <title>Change Password</title>
              </head>
              <body>
              {msg_html}
                <form action="/admin/password" method="post">
                  <label
                    >Current password
                    <input
                      type="password"
                      placeholder="Enter current password"
                      name="current_password"
                    />
                  </label>
                  <br />
                  <label
                    >New password
                    <input
                      type="password"
                      placeholder="Enter new password"
                      name="new_password"
                    />
                  </label>
                  <br />
                  <label
                    >Confirm new password
                    <input
                      type="password"
                      placeholder="Type the new password again"
                      name="new_password_check"
                    />
                  </label>
                  <br />
                  <button type="submit">Submit</button>
                </form>
              </body>
            </html>
"#,
                )),
            )
                .into_response())
        }
    }
}
