use anyhow::Context;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
};
use hmac::{Hmac, Mac};
use hyper::StatusCode;
use secrecy::ExposeSecret;
use serde::Deserialize;
use sha3::Sha3_256;

use crate::appstate::{AppState, HmacSecret};

pub async fn login_form(
    Query(query): Query<QueryParams>,
    State(shared_state): State<AppState>,
) -> impl IntoResponse {
    let error_html = match query.verify(&shared_state.hmac_secret) {
        Ok(error_message) => format!(
            "<p><i>{}</i></p>",
            htmlescape::encode_minimal(error_message.as_str())
        ),
        Err(e) => {
            tracing::warn!(error.message = %e, error.cause_chain = ?e, "Failed to verify query parameters using the HMAC tag");
            "".into()
        }
    };

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
    pub tag: Option<String>,
}

impl QueryParams {
    fn verify(mut self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag.take().context("tag is missing")?)?;
        let error = self.error.take().context("error message is missing")?;
        let error_message = urlencoding::encode(error.as_str());

        let mut mac = Hmac::<Sha3_256>::new_from_slice(secret.expose_secret().as_bytes())?;
        mac.update(error_message.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(error)
    }
}
