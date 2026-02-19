use std::env;

pub const RATE_LIMIT_DELAY_MS: u64 = 100;
pub const RECONNECT_DELAY_SECS: u64 = 5;
pub const MAX_API_RETRIES: u32 = 3;
pub const RETRY_BASE_DELAY_MS: u64 = 100;

#[derive(Debug, Clone)]
pub struct Config {
    pub max_streams_per_user: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_streams_per_user: env::var("MAX_STREAMS_PER_USER")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
        }
    }
}
