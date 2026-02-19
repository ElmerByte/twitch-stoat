# Twitch Stream Notification Bot for Stoat

A Rust bot that sends real-time Stoat notifications when Twitch streamers go live.
- ## Commands
  
  All commands require server owner permissions.
  
  ```
  !addstream <channel>           Add a Twitch channel
  !addstream <channel> <message> Add with custom message
  !removestream <channel>        Remove a channel
  !liststreams                   List monitored channels
  !helpstream                    Show help
  ```
  
  **Custom message placeholders:**
  `{channel}` - Streamer's display name
  `{url}` - Stream URL
  
  **Example:**
  
  ```
  !addstream cool_twitch_channel
  !addstream mychannel {channel} is live! Watch: {url}
  ```
- ## Installation
- ### Prerequisites
  Rust 1.93.1 or later
  Stoat bot token
  Twitch application credentials
- ## Setup
- ### 1. Create Stoat Bot
  
  Visit [stoat](https://stoat.chat) and create a bot in settings. Copy the token.
- ###  2. Get Twitch Tokens
  **Option B - Manual:**
  
  Go to [dev.twitch.tv/console/apps](https://dev.twitch.tv/console/apps)
  Register your application
  Copy Client ID
  
  Oauth: [Twitch Docs](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/)
  
  **Option A - Easy (Good for testing):**
  
  Visit [twitchtokengenerator.com](https://twitchtokengenerator.com/) (not an official twitch website use at your own risk) and generate a token with no scopes (they aren't required because stream.online/offline info is public)
  Copy: 
  * ACCESS TOKEN
  * CLIENT ID
- ### 3. Configure
  
  ```
  git clone https://github.com/ElmerByte/twitch-stoat
  cd twitch-stoat-bot
  cp .env.example .env
  ```
  
  Edit`.env`:
  
  ```
  STOAT_TOKEN=your_stoat_bot_token
  TWITCH_BOT_TOKEN=your_twitch_oauth_token
  TWITCH_CLIENT_ID=your_twitch_client_id
  MAX_STREAMS_PER_USER=3
  ```
  | Variable | Required | Default | Description | 
  | --- | --- | --- | --- |
  | `STOAT_TOKEN` | Yes | - |  Stoat bot token | 
  | `TWITCH_BOT_TOKEN` | Yes | - |  Twitch OAuth token | 
  | `TWITCH_CLIENT_ID` |  Yes |  - |  Twitch Client ID | 
  | `MAX_STREAMS_PER_USER` |  No |  3 |  Maximum streams per user |
- ## Production
  
  ```
  cargo build --release
  ./target/release/stoat-bot
  ```
- ## Architecture
  
  The bot connects to Twitch EventSub via WebSocket, subscribes to stream events, and posts notifications to configured Stoat channels when streamers go live.
  
  **Database Schema:**
  
  ```
  CREATE TABLE streams (
  id INTEGER PRIMARY KEY,
  user_id TEXT NOT NULL,
  channel_name TEXT NOT NULL,
  added_in_channel TEXT NOT NULL,
  date TEXT NOT NULL,
  custom_message TEXT,
  UNIQUE(channel_name, added_in_channel, user_id)
  );
  ```
- ## Security
  This is an early version of the bot, use it at your own risk!
- ## Troubleshooting
  
  **Bot doesn't start:**
  Verify tokens in `.env`
  Check file permissions: `chmod 600 .env`
  Review logs: `journalctl -u stoat-bot -f`
  
  **Commands don't work:**
  Verify you're the server owner
  Ensure bot is in the server
  Use commands in text channels only
  
  **Notifications not working:**
  Check if bot have permission to type in the channel
  Verify channel is added: `!liststreams`
  Check Twitch OAuth token validity
- ## Contributing
  
  Contributions are welcome. Please open an issue before submitting pull requests.
- ## License
  
  MIT License
- ## Links
- [Stoat Documentation](https://developers.stoat.chat/)
- [Twitch EventSub](https://dev.twitch.tv/docs/eventsub/)
