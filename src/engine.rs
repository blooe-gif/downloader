use crate::{
    adaptive::{AdaptiveController, MetricsSnapshot},
    config::Config,
    dashboard::RuntimeStats,
    db, fileio,
    models::{ChunkWork, DownloadTask},
    queue,
};
use anyhow::Context;
use futures_util::StreamExt;
use rand::prelude::IndexedRandom;
use reqwest::{Client, StatusCode, header};
use sqlx::SqlitePool;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
use tokio::{
    sync::Mutex,
    time::{Duration, Instant, sleep},
};

#[derive(Clone)]
pub struct Downloader {
    pub cfg: Config,
    pub pool: SqlitePool,
}

impl Downloader {
    pub fn new(cfg: Config, pool: SqlitePool) -> Self {
        Self { cfg, pool }
    }

    pub async fn enqueue(
        &self,
        url: &str,
        filename: Option<String>,
        priority: f64,
    ) -> anyhow::Result<i64> {
        let meta = probe(url).await?;
        let output_name = filename.unwrap_or_else(|| infer_filename(url));
        let mut out = PathBuf::from(&self.cfg.download_dir);
        out.push(output_name);
        let id = db::insert_task(
            &self.pool,
            url,
            &out.to_string_lossy(),
            meta.length as i64,
            meta.accept_ranges,
            priority,
        )
        .await?;

        let ranges = split_ranges(
            meta.length,
            if meta.accept_ranges {
                self.cfg.chunk_size_bytes()
            } else {
                meta.length
            },
        );
        db::upsert_chunks(&self.pool, id, &ranges).await?;
        Ok(id)
    }

    pub async fn run_next(&self) -> anyhow::Result<()> {
        let Some(task) = queue::next_task(&self.pool).await? else {
            println!("queue empty");
            return Ok(());
        };
        self.run_task(task).await
    }

    pub async fn run_task(&self, task: DownloadTask) -> anyhow::Result<()> {
        let output_path = PathBuf::from(&task.output_path);
        let file = fileio::create_preallocated(&output_path, task.file_size as u64)?;
        let file = Arc::new(file);

        let client = build_client(&self.cfg, None)?;
        let controller = AdaptiveController::new(
            self.cfg.base_connections,
            self.cfg.min_connections,
            self.cfg.max_connections,
        );
        let stats = RuntimeStats::default();
        crate::dashboard::spawn_dashboard(stats.clone()).await;

        let chunks = db::load_chunks(&self.pool, task.id).await?;
        let mut work = VecDeque::new();
        for c in chunks {
            if c.status == "done" {
                continue;
            }
            work.push_back(ChunkWork {
                chunk_id: c.id,
                start: c.start_offset as u64,
                end: c.end_offset as u64,
                downloaded: c.downloaded as u64,
                retries: c.retries as u32,
            });
        }

        let queue = Arc::new(Mutex::new(work));
        let metric_buffer = Arc::new(Mutex::new(Vec::<MetricsSnapshot>::new()));

        let tune_controller = controller.clone();
        let tune_metrics = metric_buffer.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                let mut data = tune_metrics.lock().await;
                if data.is_empty() {
                    continue;
                }
                let n = data.len() as f64;
                let agg = MetricsSnapshot {
                    throughput_mbps: data.iter().map(|m| m.throughput_mbps).sum::<f64>() / n,
                    avg_rtt_ms: data.iter().map(|m| m.avg_rtt_ms).sum::<f64>() / n,
                    error_rate: data.iter().map(|m| m.error_rate).sum::<f64>() / n,
                };
                data.clear();
                tune_controller.tune(&agg);
            }
        });

        let mut handles = Vec::new();
        for worker_id in 0..self.cfg.max_connections {
            let state = WorkerState {
                worker_id,
                task_id: task.id,
                url: task.url.clone(),
                pool: self.pool.clone(),
                file: file.clone(),
                queue: queue.clone(),
                cfg: self.cfg.clone(),
                client: client.clone(),
                controller: controller.clone(),
                stats: stats.clone(),
                metric_buffer: metric_buffer.clone(),
            };
            handles.push(tokio::spawn(async move { worker_loop(state).await }));
        }

        for h in handles {
            h.await??;
        }

        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM chunks WHERE task_id = ? AND status != 'done'",
        )
        .bind(task.id)
        .fetch_one(&self.pool)
        .await?;

        if remaining == 0 {
            db::mark_task_status(&self.pool, task.id, "completed", None).await?;
        } else {
            queue::dead_letter(&self.pool, task.id, "incomplete chunks after workers exit").await?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct WorkerState {
    worker_id: usize,
    task_id: i64,
    url: String,
    pool: SqlitePool,
    file: Arc<std::fs::File>,
    queue: Arc<Mutex<VecDeque<ChunkWork>>>,
    cfg: Config,
    client: Client,
    controller: AdaptiveController,
    stats: RuntimeStats,
    metric_buffer: Arc<Mutex<Vec<MetricsSnapshot>>>,
}

async fn worker_loop(state: WorkerState) -> anyhow::Result<()> {
    loop {
        if state.worker_id >= state.controller.target() {
            sleep(Duration::from_millis(250)).await;
            continue;
        }
        let Some(mut job) = state.queue.lock().await.pop_front() else {
            break;
        };

        state.stats.active_workers.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        let result = download_chunk(&state, &mut job).await;
        state.stats.active_workers.fetch_sub(1, Ordering::Relaxed);

        let elapsed = started.elapsed().as_secs_f64();
        let throughput_mbps = (job.downloaded as f64 / elapsed.max(0.001)) / (1024.0 * 1024.0);
        let metric = MetricsSnapshot {
            throughput_mbps,
            avg_rtt_ms: elapsed * 1000.0,
            error_rate: if result.is_ok() { 0.0 } else { 1.0 },
        };
        state.metric_buffer.lock().await.push(metric);

        if let Err(e) = result {
            state.stats.inc_error();
            job.retries += 1;
            if job.retries <= state.cfg.max_retries {
                let backoff = ((2u64.pow(job.retries.min(8))) * 50) + rand::random_range(10..120);
                sleep(Duration::from_millis(backoff)).await;
                db::mark_chunk_progress(
                    &state.pool,
                    job.chunk_id,
                    job.downloaded as i64,
                    "queued",
                    job.retries as i64,
                )
                .await?;
                state.queue.lock().await.push_back(job);
            } else {
                db::mark_chunk_progress(
                    &state.pool,
                    job.chunk_id,
                    job.downloaded as i64,
                    "failed",
                    job.retries as i64,
                )
                .await?;
                tracing::error!("chunk {} failed: {e:?}", job.chunk_id);
            }
        }
    }
    Ok(())
}

async fn download_chunk(state: &WorkerState, job: &mut ChunkWork) -> anyhow::Result<()> {
    if state.cfg.request_delay_ms > 0 {
        sleep(Duration::from_millis(state.cfg.request_delay_ms)).await;
    }

    let start = job.start + job.downloaded;
    if start > job.end {
        db::mark_chunk_progress(
            &state.pool,
            job.chunk_id,
            job.downloaded as i64,
            "done",
            job.retries as i64,
        )
        .await?;
        return Ok(());
    }

    let mut req = state
        .client
        .get(&state.url)
        .header(header::RANGE, format!("bytes={start}-{}", job.end));

    if let Some(ua) = state.cfg.user_agents.choose(&mut rand::rng()) {
        req = req.header(header::USER_AGENT, ua.as_str());
    }

    let resp = req.send().await?;
    if resp.status() == StatusCode::TOO_MANY_REQUESTS
        || resp.status() == StatusCode::SERVICE_UNAVAILABLE
    {
        if let Some(wait) = resp
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
        {
            if let Ok(seconds) = wait.parse::<u64>() {
                sleep(Duration::from_secs(seconds)).await;
            }
        }
        anyhow::bail!("throttled with {}", resp.status());
    }

    resp.error_for_status_ref()?;
    let mut stream = resp.bytes_stream();
    let mut offset = start;
    let mut dirty = 0u64;
    db::mark_chunk_progress(
        &state.pool,
        job.chunk_id,
        job.downloaded as i64,
        "downloading",
        job.retries as i64,
    )
    .await?;

    while let Some(next) = stream.next().await {
        let chunk = next?;
        let file = state.file.clone();
        let offset_now = offset;
        let data = chunk.to_vec();
        tokio::task::spawn_blocking(move || fileio::write_all_at(&file, offset_now, &data))
            .await??;

        offset += data.len() as u64;
        job.downloaded += data.len() as u64;
        dirty += data.len() as u64;
        state.stats.add_bytes(data.len() as u64);

        if dirty > (1024 * 1024) as u64 {
            db::mark_chunk_progress(
                &state.pool,
                job.chunk_id,
                job.downloaded as i64,
                "downloading",
                job.retries as i64,
            )
            .await?;
            dirty = 0;
        }
    }

    db::mark_chunk_progress(
        &state.pool,
        job.chunk_id,
        job.downloaded as i64,
        "done",
        job.retries as i64,
    )
    .await?;
    Ok(())
}

struct ProbeMeta {
    length: u64,
    accept_ranges: bool,
}

async fn probe(url: &str) -> anyhow::Result<ProbeMeta> {
    let client = Client::new();
    let head = client.head(url).send().await?;
    let accept_ranges = head
        .headers()
        .get(header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("bytes"))
        .unwrap_or(false);

    let length = head
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|n| n.parse::<u64>().ok())
        .context("missing content-length from HEAD response")?;

    Ok(ProbeMeta {
        length,
        accept_ranges,
    })
}

fn split_ranges(total: u64, chunk_size: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + chunk_size).min(total) - 1;
        out.push((start, end));
        start = end + 1;
    }
    out
}

fn build_client(cfg: &Config, proxy: Option<&str>) -> anyhow::Result<Client> {
    let mut builder = reqwest::Client::builder()
        .pool_max_idle_per_host(cfg.max_connections)
        .tcp_keepalive(Duration::from_secs(30))
        .http2_adaptive_window(true)
        .http2_keep_alive_interval(Duration::from_secs(20));

    if let Some(p) = proxy.or(cfg.proxies.first().map(String::as_str)) {
        builder = builder.proxy(reqwest::Proxy::all(p)?);
    }

    Ok(builder.build()?)
}

fn infer_filename(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();
    parsed
        .as_ref()
        .and_then(|u| u.path_segments())
        .and_then(|mut seg| seg.next_back())
        .filter(|s| !s.is_empty())
        .unwrap_or("download.bin")
        .to_owned()
}
