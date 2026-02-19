use crate::config::RECONNECT_DELAY_SECS;
use crate::error::Error;
use crate::twitch::subscription::subscribe_to_channels;
use crate::twitch::types::{
    EventSubMessage, SessionReconnect, SessionWelcome, StreamOffline, StreamOnline,
};

use futures_util::StreamExt;
use parking_lot::RwLock;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub fn start_eventsub_task(
    stoat_token: String,
    twitch_bot_token: String,
    twitch_client_id: String,
    db: Pool<SqliteConnectionManager>,
    online_channels: Arc<RwLock<HashSet<String>>>,
    session_id: Arc<RwLock<Option<String>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let http_client = reqwest::Client::new();
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        spawn_signal_handler(shutdown_tx.clone());

        let mut ws_url = "wss://eventsub.wss.twitch.tv/ws".to_string();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => break,
                result = connect_async(&ws_url) => {
                    match result {
                        Ok((ws_stream, _)) => {
                            online_channels.write().clear();

                            match run_connection(
                                ws_stream,
                                &stoat_token,
                                &twitch_bot_token,
                                &twitch_client_id,
                                db.clone(),
                                online_channels.clone(),
                                session_id.clone(),
                                http_client.clone(),
                                shutdown_tx.subscribe(),
                            ).await {
                                Ok(Some(new_url)) => ws_url = new_url,
                                Ok(None) => {}
                                Err(e) => eprintln!("EventSub error: {e}"),
                            }
                        }
                        Err(e) => eprintln!("Connection failed: {e}"),
                    }

                    tokio::time::sleep(
                        tokio::time::Duration::from_secs(RECONNECT_DELAY_SECS)
                    ).await;
                }
            }
        }

        println!("✓ EventSub task stopped");
    })
}

fn spawn_signal_handler(shutdown_tx: broadcast::Sender<()>) {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = shutdown_tx.send(());
        }
    });
}

async fn run_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    stoat_token: &str,
    twitch_bot_token: &str,
    twitch_client_id: &str,
    db: Pool<SqliteConnectionManager>,
    online_channels: Arc<RwLock<HashSet<String>>>,
    session_id: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<Option<String>, Error> {
    let (_write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            _ = shutdown.recv() => return Ok(None),
            msg = read.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => return Err(Error::DatabaseError(e.to_string())),
                    None => return Ok(None),
                };

                if let Some(new_url) = handle_ws_message(
                    msg,
                    stoat_token,
                    twitch_bot_token,
                    twitch_client_id,
                    db.clone(),
                    online_channels.clone(),
                    session_id.clone(),
                    http_client.clone(),
                ).await? {
                    return Ok(Some(new_url));
                }
            }
        }
    }
}

async fn handle_ws_message(
    msg: WsMessage,
    stoat_token: &str,
    twitch_bot_token: &str,
    twitch_client_id: &str,
    db: Pool<SqliteConnectionManager>,
    online_channels: Arc<RwLock<HashSet<String>>>,
    session_id: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
) -> Result<Option<String>, Error> {
    let text = match msg {
        WsMessage::Text(t) => t,
        WsMessage::Close(_) => return Ok(None),
        WsMessage::Ping(_) | WsMessage::Pong(_) => return Ok(None),
        _ => return Ok(None),
    };

    let event_msg: EventSubMessage =
        serde_json::from_str(&text).map_err(|e| Error::DatabaseError(e.to_string()))?;

    match event_msg.metadata.message_type.as_str() {
        "session_welcome" => {
            handle_welcome(
                event_msg.payload,
                twitch_bot_token,
                twitch_client_id,
                db,
                session_id,
            )
            .await?;
        }
        "notification" => {
            handle_notification(
                event_msg.payload,
                stoat_token,
                db,
                online_channels,
                http_client,
            )
            .await?;
        }
        "session_reconnect" => {
            return handle_reconnect(event_msg.payload);
        }
        _ => {}
    }

    Ok(None)
}

async fn handle_welcome(
    payload: Value,
    twitch_bot_token: &str,
    twitch_client_id: &str,
    db: Pool<SqliteConnectionManager>,
    session_id: Arc<RwLock<Option<String>>>,
) -> Result<(), Error> {
    let welcome: SessionWelcome =
        serde_json::from_value(payload).map_err(|e| Error::DatabaseError(e.to_string()))?;

    let id = welcome.session.id;
    *session_id.write() = Some(id.clone());

    subscribe_to_channels(&id, twitch_bot_token, twitch_client_id, db).await;

    Ok(())
}

async fn handle_notification(
    payload: Value,
    stoat_token: &str,
    db: Pool<SqliteConnectionManager>,
    online_channels: Arc<RwLock<HashSet<String>>>,
    http_client: reqwest::Client,
) -> Result<(), Error> {
    let event_data = payload
        .get("event")
        .cloned()
        .ok_or_else(|| Error::DatabaseError("missing event".into()))?;

    let subscription = payload
        .get("subscription")
        .ok_or_else(|| Error::DatabaseError("missing subscription".into()))?;

    let event_type = subscription["type"].as_str().unwrap_or("");

    match event_type {
        "stream.online" => {
            handle_stream_online(event_data, stoat_token, db, online_channels, http_client).await?;
        }
        "stream.offline" => {
            handle_stream_offline(event_data, online_channels).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_stream_online(
    event_data: Value,
    stoat_token: &str,
    db: Pool<SqliteConnectionManager>,
    online_channels: Arc<RwLock<HashSet<String>>>,
    http_client: reqwest::Client,
) -> Result<(), Error> {
    let event: StreamOnline =
        serde_json::from_value(event_data).map_err(|e| Error::DatabaseError(e.to_string()))?;

    let channel = event.broadcaster_user_login.to_lowercase();

    let should_notify = {
        let mut set = online_channels.write();
        set.insert(channel.clone())
    };

    if !should_notify {
        return Ok(());
    }

    let alert_channels = get_alert_channels(db, channel.clone()).await;

    for (alert_channel, custom_message) in alert_channels {
        let message = if let Some(custom_msg) = custom_message {
            // Use custom message - replace {channel} and {url} placeholders
            custom_msg
                .replace("{channel}", &event.broadcaster_user_name)
                .replace(
                    "{url}",
                    &format!("https://twitch.tv/{}", event.broadcaster_user_login),
                )
        } else {
            // Use default message
            format!(
                "{} is now live! https://twitch.tv/{}",
                event.broadcaster_user_name, event.broadcaster_user_login
            )
        };

        let payload = serde_json::json!({ "content": message });
        let url = format!(
            "https://api.revolt.chat/channels/{}/messages",
            alert_channel
        );

        if let Err(e) = http_client
            .post(&url)
            .header("x-bot-token", stoat_token)
            .json(&payload)
            .send()
            .await
        {
            eprintln!("Failed to send notification: {e}");
        }
    }

    Ok(())
}

async fn handle_stream_offline(
    event_data: Value,
    online_channels: Arc<RwLock<HashSet<String>>>,
) -> Result<(), Error> {
    let event: StreamOffline =
        serde_json::from_value(event_data).map_err(|e| Error::DatabaseError(e.to_string()))?;

    let channel = event.broadcaster_user_login.to_lowercase();
    online_channels.write().remove(&channel);

    Ok(())
}

fn handle_reconnect(payload: Value) -> Result<Option<String>, Error> {
    let reconnect: SessionReconnect =
        serde_json::from_value(payload).map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Some(reconnect.session.reconnect_url))
}

async fn get_alert_channels(
    db: Pool<SqliteConnectionManager>,
    channel_name: String,
) -> Vec<(String, Option<String>)> {
    tokio::task::spawn_blocking(move || {
        let conn = db.get().ok()?;
        let mut stmt = conn
            .prepare("SELECT added_in_channel, custom_message FROM streams WHERE channel_name = ?1")
            .ok()?;
        let rows = stmt
            .query_map([&channel_name], |row| Ok((row.get(0)?, row.get(1)?)))
            .ok()?;
        Some(rows.filter_map(|r| r.ok()).collect())
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
}
