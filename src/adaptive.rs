use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    pub throughput_mbps: f64,
    pub avg_rtt_ms: f64,
    pub error_rate: f64,
}

#[derive(Clone)]
pub struct AdaptiveController {
    target: Arc<AtomicUsize>,
    min: usize,
    max: usize,
}

impl AdaptiveController {
    pub fn new(base: usize, min: usize, max: usize) -> Self {
        Self {
            target: Arc::new(AtomicUsize::new(base.clamp(min, max))),
            min,
            max,
        }
    }

    pub fn target(&self) -> usize {
        self.target.load(Ordering::Relaxed)
    }

    pub fn tune(&self, metric: &MetricsSnapshot) {
        let curr = self.target();
        let next = if metric.error_rate > 0.10 || metric.avg_rtt_ms > 750.0 {
            curr.saturating_sub(1)
        } else if metric.error_rate < 0.02
            && metric.avg_rtt_ms < 250.0
            && metric.throughput_mbps > 30.0
        {
            curr + 1
        } else {
            curr
        }
        .clamp(self.min, self.max);
        self.target.store(next, Ordering::Relaxed);
    }
}
