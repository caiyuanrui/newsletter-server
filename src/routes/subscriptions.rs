use axum::{extract, http::StatusCode, response::IntoResponse};

use crate::{
    domain::{self, NewSubscriber},
    utils::Data,
};

#[derive(serde::Deserialize)]
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

#[tracing::instrument(name = "Adding a new subscriber", skip(form, pool),fields(
  subscriber_email = %form.email,
  subscriber_name = %form.name
))]
pub async fn subscribe(
    extract::State(pool): extract::State<Data<sqlx::MySqlPool>>,
    form: extract::Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
INSERT INTO subscriptions (id, email, name, subscribed_at)
VALUES (?, ?, ?, ?)
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
