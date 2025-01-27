use axum::response::{Html, IntoResponse};
use hyper::StatusCode;
use tower_cookies::Cookies;
use tracing::instrument;

#[instrument(name = "Get Login Form")]
pub async fn login_form(cookies: Cookies) -> impl IntoResponse {
    let error_html = match cookies.get("_flash") {
        Some(cookie) => format!("<p><i>{}</i></p>", cookie.value()),
        None => "".into(),
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

// /// The `error` has been decoded by the `Query` extractor.
// #[derive(Debug, Deserialize)]
// pub struct QueryParams {
//     pub error: Option<String>,
//     pub tag: Option<String>,
// }

// impl QueryParams {
//     #[instrument(
//         name = "Verify the Hmac tag in the query parameters",
//         skip(self, secret),
//         fields(error = self.error, tag = self.tag)
//     )]
//     fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
//         let tag = hex::decode(self.tag.context("tag is missing")?)?;
//         let error = self.error.context("error message is missing")?;
//         let error_message = urlencoding::encode(error.as_str());

//         let mut mac = Hmac::<Sha3_256>::new_from_slice(secret.expose_secret().as_bytes())?;
//         mac.update(error_message.as_bytes());
//         mac.verify_slice(&tag)?;

//         Ok(error)
//     }
// }
