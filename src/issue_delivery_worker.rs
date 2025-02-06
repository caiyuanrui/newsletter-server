use std::time::Duration;

use anyhow::Context;
use sqlx::{MySqlPool, MySqlTransaction};
use tracing::{field::display, instrument};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};

pub async fn run_worker_loop_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(connection_pool, email_client).await
}

async fn worker_loop(pool: MySqlPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => tokio::time::sleep(Duration::from_secs(10)).await,
            Err(ExecutionError::TransientError(_)) => {
                tokio::time::sleep(Duration::from_secs(1)).await
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
            Err(ExecutionError::FatalError(_)) => {}
        }
    }
}

#[derive(Debug)]
pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("")]
    TransientError(#[from] sqlx::Error),
    #[error("")]
    FatalError(#[from] anyhow::Error),
}

/// This function is marked `pub` just for testing.
/// # Errors
/// There are two kinds of errore:
/// - trasient error: caused by databse
/// - fatal error: failure in parsing an invalid email
#[instrument(skip_all, fields(newsletter_issue_id, subscriber_email))]
pub async fn try_execute_task(
    pool: &MySqlPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, ExecutionError> {
    match dequeue_task(pool).await? {
        None => Ok(ExecutionOutcome::EmptyQueue),
        Some((txn, issue_id, email)) => {
            tracing::Span::current()
                .record("newsletter_issue_id", display(&issue_id))
                .record("subscriber_email", display(&email));

            match SubscriberEmail::parse(email.clone()) {
                Ok(email) => {
                    let issue = get_issue(pool, issue_id).await?;
                    if let Err(e) = email_client
                        .send_email(
                            &email,
                            &issue.title,
                            &issue.html_content,
                            &issue.text_content,
                        )
                        .await
                    {
                        tracing::error!(error.cause_chain=?e, error.message=%e,
                      "Failed to deliver issue to a confirmed subscriber. \
                      Skipping.");
                    }
                }
                Err(e) => {
                    tracing::error!(error.cause_chain=?e, error.message=%e,
                "Skipping a confirmed subscriber.\
                Their stored contact details are invalid");
                }
            }

            delete_task(txn, issue_id, &email).await?;
            Ok(ExecutionOutcome::TaskCompleted)
        }
    }
}

#[instrument(skip_all)]
async fn dequeue_task(
    pool: &MySqlPool,
) -> Result<Option<(MySqlTransaction<'_>, Uuid, String)>, ExecutionError> {
    let mut txn = pool.begin().await?;
    let r = sqlx::query!(
        r#"
      SELECT newsletter_issue_id, subscriber_email FROM issue_delivery_queue LIMIT 1 FOR UPDATE SKIP LOCKED
      "#
    ).fetch_optional(&mut *txn).await.context("Failed to execute query")?;
    match r {
        Some(r) => Ok(Some((
            txn,
            Uuid::from_slice(r.newsletter_issue_id.as_slice())
                .context("Failed to parse newsletter_issue_id into uuid")?,
            r.subscriber_email,
        ))),
        None => Ok(None),
    }
}

#[instrument(skip_all)]
async fn delete_task(
    mut txn: MySqlTransaction<'_>,
    issue_id: Uuid,
    email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
      DELETE FROM issue_delivery_queue
      WHERE newsletter_issue_id = ? AND subscriber_email = ?
      "#,
        issue_id,
        email
    )
    .execute(&mut *txn)
    .await?;
    txn.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_issue(pool: &MySqlPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    sqlx::query_as!(NewsletterIssue,
        r#"SELECT title, text_content, html_content FROM newsletter_issues WHERE newsletter_issue_id = ?"#,
        issue_id
    ).fetch_one(pool).await.map(Ok)?
}
