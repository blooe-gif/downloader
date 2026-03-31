use crate::models::{ChunkState, DownloadTask};
use anyhow::Context;
use chrono::Utc;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn init_db(path: &str) -> anyhow::Result<SqlitePool> {
    let url = format!("sqlite://{}", path);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .with_context(|| format!("failed to connect db at {url}"))?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS download_tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  url TEXT NOT NULL,
  output_path TEXT NOT NULL,
  file_size INTEGER NOT NULL,
  supports_ranges INTEGER NOT NULL,
  priority REAL NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS chunks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER NOT NULL,
  start_offset INTEGER NOT NULL,
  end_offset INTEGER NOT NULL,
  downloaded INTEGER NOT NULL,
  status TEXT NOT NULL,
  retries INTEGER NOT NULL,
  UNIQUE(task_id, start_offset, end_offset)
);
"#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn insert_task(
    pool: &SqlitePool,
    url: &str,
    output_path: &str,
    file_size: i64,
    supports_ranges: bool,
    priority: f64,
) -> anyhow::Result<i64> {
    let now = Utc::now();
    let id = sqlx::query_scalar(
        r#"
INSERT INTO download_tasks(url, output_path, file_size, supports_ranges, priority, status, created_at, updated_at)
VALUES(?, ?, ?, ?, ?, 'queued', ?, ?) RETURNING id;
"#,
    )
    .bind(url)
    .bind(output_path)
    .bind(file_size)
    .bind(if supports_ranges { 1 } else { 0 })
    .bind(priority)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn fetch_task(pool: &SqlitePool, task_id: i64) -> anyhow::Result<DownloadTask> {
    let t = sqlx::query_as::<_, DownloadTask>("SELECT * FROM download_tasks WHERE id = ?")
        .bind(task_id)
        .fetch_one(pool)
        .await?;
    Ok(t)
}

pub async fn list_tasks(pool: &SqlitePool) -> anyhow::Result<Vec<DownloadTask>> {
    Ok(sqlx::query_as::<_, DownloadTask>(
        "SELECT * FROM download_tasks ORDER BY priority DESC, created_at ASC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn upsert_chunks(
    pool: &SqlitePool,
    task_id: i64,
    ranges: &[(u64, u64)],
) -> anyhow::Result<()> {
    for (start, end) in ranges {
        sqlx::query(
            r#"INSERT OR IGNORE INTO chunks(task_id, start_offset, end_offset, downloaded, status, retries)
            VALUES(?, ?, ?, 0, 'queued', 0)"#,
        )
        .bind(task_id)
        .bind(*start as i64)
        .bind(*end as i64)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn load_chunks(pool: &SqlitePool, task_id: i64) -> anyhow::Result<Vec<ChunkState>> {
    Ok(sqlx::query_as::<_, ChunkState>(
        "SELECT * FROM chunks WHERE task_id = ? ORDER BY start_offset ASC",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?)
}

pub async fn mark_task_status(
    pool: &SqlitePool,
    task_id: i64,
    status: &str,
    err: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE download_tasks SET status = ?, error = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(err)
        .bind(Utc::now())
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_chunk_progress(
    pool: &SqlitePool,
    chunk_id: i64,
    downloaded: i64,
    status: &str,
    retries: i64,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE chunks SET downloaded = ?, status = ?, retries = ? WHERE id = ?")
        .bind(downloaded)
        .bind(status)
        .bind(retries)
        .bind(chunk_id)
        .execute(pool)
        .await?;
    Ok(())
}
