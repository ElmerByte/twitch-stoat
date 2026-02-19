use crate::commands::CmdCtx;
use crate::error::Error;
use crate::twitch::unsubscribe_single_channel;
use rusqlite::params;
use stoat::MessageExt;

pub async fn removestream(ctx: CmdCtx) -> Result<(), Error> {
    let message_text = ctx.message.content.as_ref().unwrap_or(&String::new()).clone();
    let parts: Vec<&str> = message_text.split_whitespace().collect();
    
    if parts.len() < 2 {
        ctx.message
            .reply(&ctx, true)
            .content("Usage: !removestream <channel_name>".to_string())
            .build()
            .await?;
        return Ok(());
    }
    
    let channel_name = parts[1].to_lowercase();
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
                    .content("You must be the server owner to remove streams here.".to_string())
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
    
    let db = ctx.state.db.clone();
    let channel_name_clone = channel_name.clone();
    let user_id_clone = user_id.clone();
    let added_in_channel_clone = added_in_channel.clone();
    
    let delete_result = tokio::task::spawn_blocking(move || -> Result<bool, Error> {
        let conn = db.get()?;
        
        // Check if stream exists and belongs to this user
        let mut stmt = conn.prepare("SELECT user_id FROM streams WHERE channel_name = ?1 AND added_in_channel = ?2")?;
        let owner_id: Result<String, rusqlite::Error> = stmt.query_row(params![channel_name_clone, added_in_channel_clone], |row| row.get(0));
        
        match owner_id {
            Ok(owner) if owner == user_id_clone => {
                // User owns this stream, delete it
                let rows = conn.execute(
                    "DELETE FROM streams WHERE channel_name = ?1 AND added_in_channel = ?2 AND user_id = ?3",
                    params![channel_name_clone, added_in_channel_clone, user_id_clone],
                )?;
                Ok(rows > 0)
            }
            Ok(_) => Err(Error::DatabaseError("Not owner".to_string())),
            Err(_) => Err(Error::DatabaseError("Not found".to_string())),
        }
    }).await
    .map_err(|e| Error::DatabaseError(format!("Task failed: {}", e)))?;
    
    match delete_result {
        Ok(true) => {
            // Check if this channel is used in other servers
            let db = ctx.state.db.clone();
            let channel_name_clone = channel_name.clone();
            
            let count: i64 = tokio::task::spawn_blocking(move || -> Result<i64, Error> {
                let conn = db.get()?;
                let mut stmt = conn.prepare("SELECT COUNT(*) FROM streams WHERE channel_name = ?1")?;
                Ok(stmt.query_row(params![channel_name_clone], |row| row.get(0))?)
            }).await
            .map_err(|e| Error::DatabaseError(format!("Task failed: {}", e)))??;
            
            // Only delete EventSub subscription if no other servers are using it
            if count == 0 {
                unsubscribe_single_channel(
                    &channel_name,
                    &ctx.state.twitch_token,
                    &ctx.state.client_id,
                ).await;
            }
            
            println!("✓ Removed stream: {}", channel_name);
            ctx.message
                .reply(&ctx, true)
                .content(format!("Removed channel: {}", channel_name))
                .build()
                .await?;
        }
        Ok(false) => {
            ctx.message
                .reply(&ctx, true)
                .content("Stream not found.".to_string())
                .build()
                .await?;
        }
        Err(e) if e.to_string().contains("Not owner") => {
            ctx.message
                .reply(&ctx, true)
                .content("You can only remove streams you added.".to_string())
                .build()
                .await?;
        }
        Err(e) if e.to_string().contains("Not found") => {
            ctx.message
                .reply(&ctx, true)
                .content(format!("Stream {} not found in this channel.", channel_name))
                .build()
                .await?;
        }
        Err(e) => {
            eprintln!("✗ Error removing stream: {}", e);
            ctx.message
                .reply(&ctx, true)
                .content("Failed to remove stream.".to_string())
                .build()
                .await?;
        }
    }
    
    Ok(())
}
