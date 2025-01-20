use axum::{
    extract::{rejection::FormRejection, FromRequest, State},
    http,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    appstate::AppState,
    domain::{self, NewSubscriber},
    email_client::EmailClient,
};

#[axum_macros::debug_handler]
#[tracing::instrument(
  name = "Adding a new subscriber",
  skip(form, shared_state),
  fields(
  subscriber_email = %form.email,
  subscriber_name = %form.name
))]
pub async fn subscribe(
    State(shared_state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let db_pool = &shared_state.db_pool;
    let email_client = &shared_state.email_client;

    let new_subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(e) => {
            tracing::error!("{e}");
            return http::StatusCode::BAD_REQUEST;
        }
    };

    if let Err(e) = insert_subscriber(db_pool, &new_subscriber).await {
        tracing::error!("{e}");
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    };

    if let Err(e) = send_confirmation_email(email_client, &new_subscriber).await {
        tracing::error!("{e}");
        return http::StatusCode::INTERNAL_SERVER_ERROR;
    }

    http::StatusCode::OK
}

pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
) -> Result<reqwest::Response, reqwest::Error> {
    let confirmation_link = "https://my-api.com/subscriptions/confirm";
    let html_content = format!(
        r#"Welcome to our newsletter!<br />
    Click <a href="{confirmation_link}">here</a> to confirm your subscription."#,
    );
    let text_content = format!(
        r#"Welcome to our newsletter!
    Visit {confirmation_link} to confirm your subscription."#,
    );

    email_client
        .send_email(
            &new_subscriber.email,
            "Welcome",
            &html_content,
            &text_content,
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
pub async fn insert_subscriber(
    pool: &sqlx::MySqlPool,
    new_subscriber: &NewSubscriber,
) -> Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> {
    sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at, status)
VALUES (?, ?, ?, ?, 'confirmed')
"#,
        uuid::Uuid::new_v4().to_string(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = domain::SubscriberName::parse(form.name)?;
        let email = domain::SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { email, name })
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Form), rejection(ApiError))]
pub struct Form<T>(T);

#[derive(Debug)]
pub struct ApiError {
    status: http::StatusCode,
    message: String,
}

impl From<FormRejection> for ApiError {
    fn from(rejection: FormRejection) -> Self {
        Self {
            // failed to extract into Form
            status: if rejection.status() == http::StatusCode::UNPROCESSABLE_ENTITY {
                http::StatusCode::NOT_FOUND
            } else {
                rejection.status()
            },
            message: rejection.body_text(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let payload = json!({
          "message": self.message,
          "origin": "derive_from_request"
        });

        (self.status, axum::Json(payload)).into_response()
    }
}
