use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub db_path: String,
    pub download_dir: String,
    pub max_connections: usize,
    pub base_connections: usize,
    pub min_connections: usize,
    pub chunk_size_mb: u64,
    pub user_agents: Vec<String>,
    pub proxies: Vec<String>,
    pub request_delay_ms: u64,
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: "idm_state.sqlite".into(),
            download_dir: "downloads".into(),
            max_connections: 32,
            base_connections: 8,
            min_connections: 2,
            chunk_size_mb: 8,
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".into(),
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".into(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15".into(),
            ],
            proxies: vec![],
            request_delay_ms: 0,
            max_retries: 8,
        }
    }
}

impl Config {
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let body = fs::read_to_string(path)?;
            Ok(toml::from_str(&body)?)
        } else {
            let default = Self::default();
            fs::write(path, toml::to_string_pretty(&default)?)?;
            Ok(default)
        }
    }

    pub fn chunk_size_bytes(&self) -> u64 {
        self.chunk_size_mb * 1024 * 1024
    }
}
