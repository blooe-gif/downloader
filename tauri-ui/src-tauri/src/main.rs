#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use idm_rs::{ai, config::Config, db, engine::Downloader};
use serde::Serialize;
use sqlx::SqlitePool;
use std::{collections::HashMap, path::PathBuf};
use tauri::State;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppCore {
    pool: SqlitePool,
    downloader: Downloader,
}

#[derive(Default)]
struct UiState {
    core: Mutex<Option<AppCore>>,
}

#[derive(Serialize)]
struct TaskRow {
    id: i64,
    status: String,
    priority: f64,
    file_size: i64,
    output_path: String,
}

#[tauri::command]
async fn list_tasks_cmd(state: State<'_, UiState>) -> Result<Vec<TaskRow>, String> {
    let core = get_core(&state).await?;
    let tasks = db::list_tasks(&core.pool).await.map_err(err)?;
    Ok(tasks
        .into_iter()
        .map(|t| TaskRow {
            id: t.id,
            status: t.status,
            priority: t.priority,
            file_size: t.file_size,
            output_path: t.output_path,
        })
        .collect())
}

#[tauri::command]
async fn queue_task_cmd(
    state: State<'_, UiState>,
    url: String,
    output: Option<String>,
) -> Result<i64, String> {
    let core = get_core(&state).await?;
    let priority = score_priority(&url, output.as_deref().unwrap_or(""));
    core.downloader
        .enqueue(&url, output, priority)
        .await
        .map_err(err)
}

#[tauri::command]
async fn run_next_cmd(state: State<'_, UiState>) -> Result<String, String> {
    let core = get_core(&state).await?;
    core.downloader.run_next().await.map_err(err)?;
    Ok("Run complete.".into())
}

async fn get_core(state: &State<'_, UiState>) -> Result<AppCore, String> {
    let mut guard = state.core.lock().await;
    if let Some(core) = guard.clone() {
        return Ok(core);
    }

    let config_path = PathBuf::from("../../idm.toml");
    let cfg = Config::load_or_create(&config_path).map_err(err)?;
    let pool = db::init_db(&cfg.db_path).await.map_err(err)?;
    let core = AppCore {
        pool: pool.clone(),
        downloader: Downloader::new(cfg, pool),
    };

    *guard = Some(core.clone());
    Ok(core)
}

fn score_priority(url: &str, output: &str) -> f64 {
    let mut rules = HashMap::new();
    rules.insert("critical".to_string(), 8.0);
    rules.insert("backup".to_string(), 4.0);
    rules.insert("patch".to_string(), 5.5);
    ai::priority_score(url, output, &rules)
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn main() {
    tauri::Builder::default()
        .manage(UiState::default())
        .invoke_handler(tauri::generate_handler![
            list_tasks_cmd,
            queue_task_cmd,
            run_next_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
