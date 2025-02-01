use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use hyper::{header, StatusCode};
use tower_cookies::{cookie::time::Duration, Cookie, Cookies};

use crate::{
    appstate::HmacSecret,
    routes::SignedCookieValue,
    session_state::TypedSession,
    utils::{e500, see_other},
};

pub async fn change_password_form(
    session: TypedSession,
    cookies: Cookies,
    State(secret): State<HmacSecret>,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        None => Ok(see_other("/login")),
        Some(_user_id) => {
            let msg_html = cookies
                .get("_flash")
                .and_then(|cookie| serde_json::from_str::<SignedCookieValue>(cookie.value()).ok())
                .filter(|value| value.validate(&secret))
                .map(|value| format!("<p><i>{}</i></p>", value.message))
                .unwrap_or_default();

            let new_value = SignedCookieValue::new("".into(), &secret);
            let cookie = Cookie::build(("_flash", new_value.into_json()))
                .max_age(Duration::ZERO)
                .http_only(true)
                .secure(true)
                .build();

            Ok((
                StatusCode::OK,
                [(header::SET_COOKIE, cookie.to_string())],
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
