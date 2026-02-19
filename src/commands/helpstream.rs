use crate::commands::CmdCtx;
use crate::error::Error;
use stoat::MessageExt;

pub async fn helpstream(ctx: CmdCtx) -> Result<(), Error> {
    let help_text = r#"**Stream Notification Bot**

**Commands** (Server owner only):

`!addstream <channel>` - Monitor a Twitch channel
`!addstream <channel> <message>` - Monitor with custom notification
`!removestream <channel>` - Stop monitoring a channel
`!liststreams` - View monitored channels
`!helpstream` - Show this help message

**Custom Messages:**
Use `{channel}` for streamer name and `{url}` for stream link.

**Example:**
`!addstream mychannel 🔴 {channel} is live! {url}`"#;

    ctx.message
        .reply(&ctx, true)
        .content(help_text.to_string())
        .build()
        .await?;

    Ok(())
}
