# IDM-RS: Production-grade Rust Download Accelerator

IDM-RS is a high-performance, resumable, multi-connection downloader with adaptive concurrency, persistent state, retry logic, and priority scheduling.

## Features

- Multi-chunk range downloader (8-32+ workers per file)
- Persistent chunk state in SQLite, resume-safe after process restart
- Adaptive concurrency controller based on throughput/RTT/error metrics
- Exponential backoff with jitter and `Retry-After` support
- File preallocation + random-access writes (`write_at`) for low overhead merging
- User-agent rotation, optional proxying, optional request delay
- Queueing with AI-inspired lightweight relevance scoring
- Live CLI telemetry dashboard (speed/active workers/errors)
- Tauri Windows UI scaffold under `tauri-ui/`

## CLI

```bash
cargo run -- add https://example.com/file.iso --output file.iso
cargo run -- list
cargo run -- run
cargo run -- run-task 1
```

## Config

Copy `idm.example.toml` to `idm.toml` and tune values.

## Architecture

- `src/engine.rs`: chunk scheduler, worker pool, adaptive scaling
- `src/db.rs`: SQLite persistence for tasks/chunks
- `src/fileio.rs`: preallocation and offset writes
- `src/adaptive.rs`: dynamic connection target controller
- `src/dashboard.rs`: runtime metrics stream
- `src/ai.rs`: local lightweight embeddings/scoring utilities
- `src/queue.rs`: prioritized queue + dead-letter path

