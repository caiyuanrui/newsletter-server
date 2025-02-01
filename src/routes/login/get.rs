use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use hmac::{Hmac, Mac};
use hyper::{header, StatusCode};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tower_cookies::{cookie::time::Duration, Cookie, Cookies};
use tracing::instrument;

use crate::appstate::HmacSecret;

#[instrument(name = "Get Login Form", skip(hmac_secret))]
pub async fn login_form(
    cookies: Cookies,
    State(hmac_secret): State<HmacSecret>,
) -> impl IntoResponse {
    let error_html: String = cookies
        .get("_flash")
        .and_then(|cookie| serde_json::from_str::<SignedCookieValue>(cookie.value()).ok())
        .filter(|value| value.validate(&hmac_secret))
        .map(|value| format!("<p><i>{}</i></p>", value.message))
        .unwrap_or_default();

    let new_value = SignedCookieValue::new("".into(), &hmac_secret);
    let cookie = Cookie::build(("_flash", new_value.into_json()))
        .max_age(Duration::ZERO)
        .http_only(true)
        .secure(true)
        .build();

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie.to_string())],
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedCookieValue {
    pub message: String,
    pub tag: String,
}

impl SignedCookieValue {
    pub fn new(message: String, key: &HmacSecret) -> Self {
        let mut mac =
            Hmac::<sha3::Sha3_256>::new_from_slice(key.expose_secret().as_bytes()).unwrap();
        mac.update(message.as_bytes());
        let result = mac.finalize().into_bytes();
        let tag = format!("{:x}", result);
        Self { message, tag }
    }

    pub fn validate(&self, key: &HmacSecret) -> bool {
        let mut mac =
            Hmac::<sha3::Sha3_256>::new_from_slice(key.expose_secret().as_bytes()).unwrap();
        mac.update(self.message.as_bytes());
        let result = mac.finalize().into_bytes();
        let tag = format!("{:x}", result);
        tag == self.tag
    }

    pub fn into_json(self) -> String {
        serde_json::json!(self).to_string()
    }

    pub fn from_json(value: &str) -> serde_json::Result<Self> {
        serde_json::de::from_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let cookie = Cookie::new("name", "value");
        println!("{}", cookie);
        let cookie = Cookie::build(("base_name", "base_value"))
            .http_only(true)
            .build();
        println!("{}", cookie);
    }
}
