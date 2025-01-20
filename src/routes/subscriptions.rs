use axum::{
    extract::{rejection::FormRejection, FromRequest, State},
    http,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    domain::{self, NewSubscriber},
    utils::Data,
};

#[tracing::instrument(name = "Adding a new subscriber", skip(form, pool),fields(
  subscriber_email = %form.email,
  subscriber_name = %form.name
))]
pub async fn subscribe(
    State(pool): State<Data<sqlx::MySqlPool>>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return http::StatusCode::BAD_REQUEST,
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => http::StatusCode::OK,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
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
