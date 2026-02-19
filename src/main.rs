mod commands;
mod config;
mod db;
mod error;
mod events;
mod state;
mod twitch;

use dotenv::dotenv;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use stoat::Client;

use config::Config;
use error::Error;
use state::State;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    // Stoat credentials
    let stoat_token = env::var("STOAT_TOKEN").expect("STOAT_TOKEN not set in .env");

    // Twitch credentials
    let twitch_bot_token = env::var("TWITCH_BOT_TOKEN").expect("TWITCH_BOT_TOKEN not set in .env");
    let twitch_client_id = env::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID not set in .env");

    let db = db::init_database()?;

    let online_channels = Arc::new(RwLock::new(HashSet::new()));
    let session_id = Arc::new(RwLock::new(None));

    let config = Config::default();
    println!("ℹ Max streams per user: {}", config.max_streams_per_user);

    let state = State {
        db: db.clone(),
        online_channels: online_channels.clone(),
        client_id: twitch_client_id.clone(),
        session_id: session_id.clone(),
        twitch_token: twitch_bot_token.clone(),
        config,
    };

    let commands = commands::create_handler(state.clone());

    let events = events::Events {
        command_handler: commands,
    };

    let mut client = Client::new(events).await?;

    // Start EventSub in background
    let eventsub_handle = twitch::start_eventsub_task(
        stoat_token.clone(),
        twitch_bot_token,
        twitch_client_id,
        db,
        online_channels,
        session_id,
    );

    // Run with graceful shutdown
    tokio::select! {
        result = client.run(&stoat_token) => {
            println!("ℹ Stoat client stopped");
            result
        }
        _ = tokio::signal::ctrl_c() => {
            println!("ℹ Shutting down...");
            eventsub_handle.abort();
            Ok(())
        }
    }
}
