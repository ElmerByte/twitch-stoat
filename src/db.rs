use crate::error::Error;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub fn init_database() -> Result<Pool<SqliteConnectionManager>, Error> {
    let manager = SqliteConnectionManager::file("streams.db");
    let pool = Pool::new(manager)?;

    let conn = pool.get()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS streams (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            channel_name TEXT NOT NULL,
            added_in_channel TEXT NOT NULL,
            date TEXT NOT NULL,
            custom_message TEXT,
            UNIQUE(channel_name, added_in_channel, user_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_channel_name ON streams(channel_name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_id ON streams(user_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_added_in_channel ON streams(added_in_channel)",
        [],
    )?;

    Ok(pool)
}
