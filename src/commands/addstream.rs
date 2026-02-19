use crate::commands::CmdCtx;
use crate::error::Error;
use crate::twitch::{validate_twitch_channel, subscribe_single_channel};
use rusqlite::params;
use stoat::MessageExt;

pub async fn addstream(ctx: CmdCtx) -> Result<(), Error> {
    let message_text = ctx.message.content.as_ref().unwrap_or(&String::new()).clone();
    let parts: Vec<&str> = message_text.split_whitespace().collect();
    
    if parts.len() < 2 {
        ctx.message
            .reply(&ctx, true)
            .content("Usage: !addstream <channel_name> [custom_message]".to_string())
            .build()
            .await?;
        return Ok(());
    }
    
    let channel_name = parts[1].to_lowercase();
    
    // Everything after channel name is custom message
    let custom_message = if parts.len() > 2 {
        Some(parts[2..].join(" "))
    } else {
        None
    };
    
    // Validate channel name
    if channel_name.is_empty() || channel_name.len() > 25 || !channel_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        ctx.message
            .reply(&ctx, true)
            .content("Invalid channel name. Must be alphanumeric or underscores, max 25 characters.".to_string())
            .build()
            .await?;
        return Ok(());
    }
    
    let user = match ctx.message.user.as_ref() {
        Some(u) => u,
        None => {
            ctx.message
                .reply(&ctx, true)
                .content("Unable to identify user.".to_string())
                .build()
                .await?;
            return Ok(());
        }
    };
    let user_id = user.id.clone();
    let added_in_channel = ctx.message.channel.clone();
    
    // Check if user is server owner (for TextChannel only)
    let channel = ctx.cache.get_channel(&added_in_channel).unwrap();
    match channel {
        stoat::types::Channel::TextChannel { server, .. } => {
            let server_obj = ctx.cache.get_server(&server).unwrap();
            if server_obj.owner != user_id {
                ctx.message
                    .reply(&ctx, true)
                    .content("You must be the server owner to add streams here.".to_string())
                    .build()
                    .await?;
                return Ok(());
            }
        }
        _ => {
            ctx.message
                .reply(&ctx, true)
                .content("This command only works in server text channels.".to_string())
                .build()
                .await?;
            return Ok(());
        }
    }
    
    // Check if user already has max streams
    let db = ctx.state.db.clone();
    let user_id_clone = user_id.clone();
    let max_streams = ctx.state.config.max_streams_per_user;
    
    let count: i64 = tokio::task::spawn_blocking(move || -> Result<i64, Error> {
        let conn = db.get()?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM streams WHERE user_id = ?1")?;
        Ok(stmt.query_row(params![user_id_clone], |row| row.get(0))?)
    }).await
    .map_err(|e| Error::DatabaseError(format!("Task failed: {}", e)))??;
    
    if count >= max_streams {
        ctx.message
            .reply(&ctx, true)
            .content(format!("You have reached the maximum limit of {} streams.", max_streams))
            .build()
            .await?;
        return Ok(());
    }
    
    // Validate that Twitch channel exists
    match validate_twitch_channel(&channel_name, &ctx.state.twitch_token, &ctx.state.client_id).await {
        Ok(Some(_broadcaster_id)) => {
            // Channel exists, continue
        }
        Ok(None) => {
            ctx.message
                .reply(&ctx, true)
                .content(format!("Twitch channel '{}' not found.", channel_name))
                .build()
                .await?;
            return Ok(());
        }
        Err(e) => {
            eprintln!("✗ Failed to validate Twitch channel: {}", e);
            ctx.message
                .reply(&ctx, true)
                .content("Failed to validate channel with Twitch API.".to_string())
                .build()
                .await?;
            return Ok(());
        }
    }
    
    let db = ctx.state.db.clone();
    let channel_name_clone = channel_name.clone();
    let added_in_channel_clone = added_in_channel.clone();
    let user_id_clone = user_id.clone();
    let custom_message_clone = custom_message.clone();
    
    let insert_result = tokio::task::spawn_blocking(move || -> Result<usize, Error> {
        let conn = db.get()?;
        let date = chrono::Utc::now().to_rfc3339();
        
        Ok(conn.execute(
            "INSERT INTO streams (user_id, channel_name, added_in_channel, date, custom_message) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_id_clone, channel_name_clone, added_in_channel_clone, date, custom_message_clone],
        )?)
    }).await
    .map_err(|e| Error::DatabaseError(format!("Task failed: {}", e)))?;
    
    match insert_result {
        Ok(_) => {
            // Subscribe to the channel in EventSub
            let session_id = ctx.state.session_id.read().clone();
            
            if let Some(session_id) = session_id {
                if let Err(e) = subscribe_single_channel(
                    &channel_name,
                    &session_id,
                    &ctx.state.twitch_token,
                    &ctx.state.client_id,
                ).await {
                    eprintln!("✗ Failed to subscribe to EventSub: {}", e);
                }
            } else {
                eprintln!("✗ EventSub session not ready yet");
            }
            
            let response = if custom_message.is_some() {
                format!("Added channel: {} (with custom message)", channel_name)
            } else {
                format!("Added channel: {}", channel_name)
            };
            
            ctx.message
                .reply(&ctx, true)
                .content(response)
                .build()
                .await?;
        }
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                ctx.message
                    .reply(&ctx, true)
                    .content("This channel is already added in this server.".to_string())
                    .build()
                    .await?;
            } else {
                eprintln!("✗ Database error adding stream: {}", e);
                ctx.message
                    .reply(&ctx, true)
                    .content("Failed to add channel.".to_string())
                    .build()
                    .await?;
            }
        }
    }
    
    Ok(())
}
