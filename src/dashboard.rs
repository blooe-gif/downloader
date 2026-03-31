use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use tokio::time::{Duration, Instant, sleep};

#[derive(Clone, Default)]
pub struct RuntimeStats {
    pub downloaded_bytes: Arc<AtomicU64>,
    pub active_workers: Arc<AtomicUsize>,
    pub errors: Arc<AtomicUsize>,
}

impl RuntimeStats {
    pub fn add_bytes(&self, n: u64) {
        self.downloaded_bytes.fetch_add(n, Ordering::Relaxed);
    }
    pub fn inc_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
}

pub async fn spawn_dashboard(stats: RuntimeStats) {
    tokio::spawn(async move {
        let mut last = 0u64;
        let mut last_t = Instant::now();
        loop {
            sleep(Duration::from_secs(1)).await;
            let now = stats.downloaded_bytes.load(Ordering::Relaxed);
            let dt = last_t.elapsed().as_secs_f64().max(0.001);
            let mbps = ((now - last) as f64 / dt) / (1024.0 * 1024.0);
            eprintln!(
                "speed={mbps:.2} MiB/s active={} downloaded={:.2} MiB errors={}",
                stats.active_workers.load(Ordering::Relaxed),
                now as f64 / (1024.0 * 1024.0),
                stats.errors.load(Ordering::Relaxed)
            );
            last = now;
            last_t = Instant::now();
        }
    });
}
