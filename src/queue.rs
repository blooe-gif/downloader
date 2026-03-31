use crate::db;
use crate::models::DownloadTask;
use sqlx::SqlitePool;

pub async fn next_task(pool: &SqlitePool) -> anyhow::Result<Option<DownloadTask>> {
    let row = sqlx::query_as::<_, DownloadTask>(
        "SELECT * FROM download_tasks WHERE status IN ('queued','paused') ORDER BY priority DESC, created_at ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    if let Some(task) = &row {
        db::mark_task_status(pool, task.id, "running", None).await?;
    }
    Ok(row)
}

pub async fn dead_letter(pool: &SqlitePool, task_id: i64, err: &str) -> anyhow::Result<()> {
    db::mark_task_status(pool, task_id, "failed", Some(err)).await
}
