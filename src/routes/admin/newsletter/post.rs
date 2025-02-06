use anyhow::Context;
use axum::{extract::State, response::Response, Extension, Form};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::{MySqlPool, MySqlTransaction};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    domain::UserId,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    utils::{e400, e500, see_other},
};

/// Store the newsletter issue into the database and persist all delivery tasks,
/// the background workers will handle all deliveries for us.
#[instrument(name = "Publish a newsletter issue", skip_all, fields(user_id=%&user_id))]
pub async fn publish_newsletter(
    messages: Messages,
    State(pool): State<MySqlPool>,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    let FormData {
        title,
        html_content,
        text_content,
        idempotency_key,
    } = form;
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    // Return early if we have a saved response in the database
    let mut txn = match try_processing(&pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(txn) => txn,
        NextAction::ReturnSavedResponse(saved_response) => {
            send_success_message(messages);
            return Ok(saved_response);
        }
    };

    let newsletter_issue_id =
        insert_newsletter_issue(&mut txn, &title, &html_content, &text_content)
            .await
            .context("Failed to store newsletter issue details")
            .map_err(e500)?;
    enqueue_delivery_tasks(&mut txn, newsletter_issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;
    let response = see_other("/admin/newsletters");
    let response = save_response(txn, &idempotency_key, user_id, response)
        .await
        .map_err(e500)?;
    send_success_message(messages);
    Ok(response)
}

fn send_success_message(messages: Messages) {
    messages.success(
        "The newsletter issue has been accepted - \
      emails will go out shortly!",
    );
}

#[instrument(skip_all)]
async fn insert_newsletter_issue(
    txn: &mut MySqlTransaction<'_>,
    title: &str,
    html_content: &str,
    text_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO newsletter_issues (
          newsletter_issue_id,
          title,
          text_content,
          html_content,
          published_at
        )
        VALUES (?, ?, ?, ?, now())"#,
        newsletter_issue_id,
        title,
        html_content,
        text_content
    )
    .execute(&mut **txn)
    .await?;
    Ok(newsletter_issue_id)
}

#[instrument(skip_all)]
async fn enqueue_delivery_tasks(
    txn: &mut MySqlTransaction<'_>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
      INSERT INTO issue_delivery_queue (
        newsletter_issue_id,
        subscriber_email
      )
      SELECT ?, email
      FROM subscriptions
      WHERE status = 'confirmed'
      "#,
        newsletter_issue_id
    )
    .execute(&mut **txn)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}
