use crate::config::RETRY_BASE_DELAY_MS;
use std::future::Future;

pub async fn retry_with_backoff<F, Fut, T, E>(mut f: F, max_retries: u32) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) if retries >= max_retries => return Err(e),
            Err(_) => {
                retries += 1;
                let delay = 2u64.pow(retries) * RETRY_BASE_DELAY_MS;
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

pub async fn validate_twitch_channel(
    channel: &str,
    twitch_token: &str,
    client_id: &str,
) -> Result<Option<String>, reqwest::Error> {
    let http_client = reqwest::Client::new();
    let url = format!("https://api.twitch.tv/helix/users?login={}", channel);
    
    let resp = http_client
        .get(&url)
        .header("Authorization", format!("Bearer {}", twitch_token))
        .header("Client-Id", client_id)
        .send()
        .await?;
    
    let data: serde_json::Value = resp.json().await?;
    Ok(data["data"][0]["id"].as_str().map(|s| s.to_string()))
}
