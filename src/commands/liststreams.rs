use crate::commands::CmdCtx;
use crate::error::Error;
use stoat::MessageExt;

pub async fn liststreams(ctx: CmdCtx) -> Result<(), Error> {
    let added_in_channel = ctx.message.channel.clone();

    // Check if this is a server text channel
    let channel = ctx.cache.get_channel(&added_in_channel).unwrap();
    match channel {
        stoat::types::Channel::TextChannel { .. } => {
            // Valid channel type
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
    let added_in_channel_clone = added_in_channel.clone();

    let streams: Vec<(String, Option<String>)> = tokio::task::spawn_blocking(move || {
        let conn = db.get()?;
        let mut stmt = conn.prepare(
            "SELECT channel_name, custom_message FROM streams WHERE added_in_channel = ?1",
        )?;
        let rows = stmt.query_map([added_in_channel_clone], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        Ok::<Vec<(String, Option<String>)>, Error>(rows.filter_map(|r| r.ok()).collect())
    })
    .await
    .map_err(|e| Error::DatabaseError(format!("Task failed: {}", e)))??;

    if streams.is_empty() {
        ctx.message
            .reply(&ctx, true)
            .content("No streams configured for this channel.".to_string())
            .build()
            .await?;
    } else {
        let mut response = format!("**Streams in this channel ({}):**\n", streams.len());
        for (channel, custom_msg) in streams {
            if custom_msg.is_some() {
                response.push_str(&format!("- {} (custom message)\n", channel));
            } else {
                response.push_str(&format!("- {}\n", channel));
            }
        }
        ctx.message
            .reply(&ctx, true)
            .content(response)
            .build()
            .await?;
    }

    Ok(())
}
