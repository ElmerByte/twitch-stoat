use crate::config::{MAX_API_RETRIES, RATE_LIMIT_DELAY_MS};
use crate::twitch::types::{CreateSubscription, Transport};
use crate::twitch::validation::retry_with_backoff;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

pub async fn subscribe_single_channel(
    channel: &str,
    session_id: &str,
    twitch_token: &str,
    client_id: &str,
) -> Result<String, String> {
    let http_client = reqwest::Client::new();

    let broadcaster_id = get_broadcaster_id(channel, twitch_token, client_id, &http_client)
        .await
        .map_err(|e| format!("Failed to get broadcaster ID: {}", e))?;

    if is_already_subscribed(&broadcaster_id, twitch_token, client_id, &http_client).await {
        println!("  ℹ {} (already subscribed)", channel);
        return Ok(broadcaster_id);
    }

    subscribe_to_event(
        "stream.online",
        &broadcaster_id,
        session_id,
        twitch_token,
        client_id,
        &http_client,
        channel,
    )
    .await?;
    subscribe_to_event(
        "stream.offline",
        &broadcaster_id,
        session_id,
        twitch_token,
        client_id,
        &http_client,
        channel,
    )
    .await?;

    Ok(broadcaster_id)
}

pub async fn unsubscribe_single_channel(channel: &str, twitch_token: &str, client_id: &str) {
    let http_client = reqwest::Client::new();

    let broadcaster_id =
        match get_broadcaster_id(channel, twitch_token, client_id, &http_client).await {
            Ok(id) => id,
            Err(e) => {
                eprintln!("✗ Failed to get broadcaster ID for {}: {}", channel, e);
                return;
            }
        };

    let subscriptions = match list_subscriptions(twitch_token, client_id, &http_client).await {
        Ok(subs) => subs,
        Err(e) => {
            eprintln!("✗ Failed to list subscriptions: {}", e);
            return;
        }
    };

    for sub in subscriptions {
        if sub["condition"]["broadcaster_user_id"].as_str() == Some(&broadcaster_id) {
            if let Some(sub_id) = sub["id"].as_str() {
                delete_subscription(sub_id, twitch_token, client_id, &http_client).await;
            }
        }
    }

    println!("✓ Unsubscribed from {}", channel);
}

pub async fn subscribe_to_channels(
    session_id: &str,
    twitch_token: &str,
    client_id: &str,
    db: Pool<SqliteConnectionManager>,
) {
    let channels = get_channels_from_db(db.clone()).await;

    if channels.is_empty() {
        println!("ℹ No channels to monitor yet");
        return;
    }

    println!("ℹ Subscribing to {} channels...", channels.len());

    for channel in channels {
        match subscribe_single_channel(&channel, session_id, twitch_token, client_id).await {
            Ok(_) => {}
            Err(_) => {
                remove_channel_from_db(&channel, db.clone()).await;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(RATE_LIMIT_DELAY_MS)).await;
    }
}

// Helper functions

async fn get_broadcaster_id(
    channel: &str,
    twitch_token: &str,
    client_id: &str,
    http_client: &reqwest::Client,
) -> Result<String, String> {
    let url = format!("https://api.twitch.tv/helix/users?login={}", channel);

    let resp = retry_with_backoff(
        || {
            let http_client = http_client.clone();
            let url = url.clone();
            let twitch_token = twitch_token.to_string();
            let client_id = client_id.to_string();
            async move {
                http_client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", twitch_token))
                    .header("Client-Id", &client_id)
                    .send()
                    .await
            }
        },
        MAX_API_RETRIES,
    )
    .await
    .map_err(|e| format!("Request error: {:?}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    data["data"][0]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            eprintln!("  ✗ {} (not found)", channel);
            format!("Channel {} not found", channel)
        })
}

async fn is_already_subscribed(
    broadcaster_id: &str,
    twitch_token: &str,
    client_id: &str,
    http_client: &reqwest::Client,
) -> bool {
    let list_url = format!(
        "https://api.twitch.tv/helix/eventsub/subscriptions?type=stream.online&user_id={}",
        broadcaster_id
    );

    let resp = match http_client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", twitch_token))
        .header("Client-Id", client_id)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };

    let subs_data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(_) => return false,
    };

    subs_data["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .any(|sub| sub["condition"]["broadcaster_user_id"].as_str() == Some(broadcaster_id))
        })
        .unwrap_or(false)
}

async fn subscribe_to_event(
    event_type: &str,
    broadcaster_id: &str,
    session_id: &str,
    twitch_token: &str,
    client_id: &str,
    http_client: &reqwest::Client,
    channel: &str,
) -> Result<(), String> {
    let subscription = CreateSubscription {
        sub_type: event_type.to_string(),
        version: "1".to_string(),
        condition: serde_json::json!({
            "broadcaster_user_id": broadcaster_id
        }),
        transport: Transport {
            method: "websocket".to_string(),
            session_id: session_id.to_string(),
        },
    };

    let resp = http_client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", twitch_token))
        .header("Client-Id", client_id)
        .json(&subscription)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if resp.status().is_success() {
        println!("  ✓ Subscribed to {} for {}", event_type, channel);
        Ok(())
    } else {
        eprintln!(
            "  ✗ Failed to subscribe to {} for {}: {}",
            event_type,
            channel,
            resp.status()
        );
        Err(format!("Subscription failed: {}", resp.status()))
    }
}

async fn list_subscriptions(
    twitch_token: &str,
    client_id: &str,
    http_client: &reqwest::Client,
) -> Result<Vec<serde_json::Value>, String> {
    let resp = http_client
        .get("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", twitch_token))
        .header("Client-Id", client_id)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;

    Ok(data["data"].as_array().cloned().unwrap_or_default())
}

async fn delete_subscription(
    sub_id: &str,
    twitch_token: &str,
    client_id: &str,
    http_client: &reqwest::Client,
) {
    let delete_url = format!(
        "https://api.twitch.tv/helix/eventsub/subscriptions?id={}",
        sub_id
    );

    match http_client
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", twitch_token))
        .header("Client-Id", client_id)
        .send()
        .await
    {
        Ok(_) => {}
        Err(e) => eprintln!("✗ Failed to delete subscription: {}", e),
    }
}

async fn get_channels_from_db(db: Pool<SqliteConnectionManager>) -> Vec<String> {
    tokio::task::spawn_blocking(move || {
        let conn = db.get().ok()?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT channel_name FROM streams")
            .ok()?;
        let rows = stmt.query_map([], |row| row.get(0)).ok()?;
        Some(rows.filter_map(|r| r.ok()).collect())
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| {
        eprintln!("✗ Failed to get channels from database");
        Vec::new()
    })
}

async fn remove_channel_from_db(channel: &str, db: Pool<SqliteConnectionManager>) {
    println!(
        "  ℹ Removing non-existent channel {} from database",
        channel
    );
    let channel = channel.to_string();

    tokio::task::spawn_blocking(move || {
        if let Ok(conn) = db.get() {
            if let Err(e) = conn.execute(
                "DELETE FROM streams WHERE channel_name = ?1",
                params![channel],
            ) {
                eprintln!("  ✗ Failed to remove channel from DB: {}", e);
            }
        }
    })
    .await
    .ok();
}
