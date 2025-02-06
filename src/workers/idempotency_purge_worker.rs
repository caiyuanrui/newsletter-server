use std::time::Duration;

use sqlx::MySqlPool;

use crate::{configuration::Settings, startup::get_connection_pool};

pub async fn run_worker_loop_until_stopped(
    config: Settings,
    expired_minutes: u64,
) -> Result<(), anyhow::Error> {
    let pool = get_connection_pool(&config.database);
    worker_loop(pool, expired_minutes).await
}

async fn worker_loop(pool: MySqlPool, expired_minutes: u64) -> Result<(), anyhow::Error> {
    loop {
        match purge_expired_idempotencies(&pool, expired_minutes).await {
            Ok(num) => {
                tracing::info!(purged_rows = num);
                tokio::time::sleep(Duration::from_secs(expired_minutes * 30)).await;
            }
            Err(e) => {
                tracing::error!(error.cause_chain=?e, error.message=%e, "Failed to execute purge transaction");
                tokio::time::sleep(Duration::from_secs(1)).await
            }
        }
    }
}

async fn purge_expired_idempotencies(
    pool: &MySqlPool,
    expired_minutes: u64,
) -> Result<u64, sqlx::Error> {
    let mut txn = pool.begin().await?;
    let num = sqlx::query!(
        r#"
      DELETE FROM idempotency
      WHERE TIMESTAMPDIFF(MINUTE, NOW(), created_at) > ?
      "#,
        expired_minutes
    )
    .execute(&mut *txn)
    .await?
    .rows_affected();
    txn.commit().await?;

    Ok(num)
}
