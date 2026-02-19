use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::collections::HashSet;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct State {
    pub db: Pool<SqliteConnectionManager>,
    #[allow(dead_code)] // Used in background EventSub task
    pub online_channels: Arc<RwLock<HashSet<String>>>,
    pub client_id: String,
    pub session_id: Arc<RwLock<Option<String>>>,
    pub twitch_token: String,
    pub config: Config,
}
