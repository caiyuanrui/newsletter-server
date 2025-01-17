use axum::{extract, http::StatusCode, response::IntoResponse};

use crate::{
    data::Data,
    domain::{self, NewSubscriber},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(name = "Adding a new subscriber", skip(form, pool),fields(
  subscriber_email = %form.email,
  subscriber_name = %form.name
))]
pub async fn subscribe(
    extract::State(pool): extract::State<Data<sqlx::MySqlPool>>,
    form: extract::Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = domain::NewSubscriber {
        email: form.0.email,
        name: domain::SubscriberName::parse(form.0.name),
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
        new_subscriber.email,
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
