use crate::{commands::CommandHandler, error::Error};
use stoat::{Context, EventHandler, async_trait, types::Message};

#[derive(Clone)]
pub struct Events {
    pub command_handler: CommandHandler,
}

#[async_trait]
impl EventHandler for Events {
    type Error = Error;

    async fn ready(&self, context: Context) -> Result<(), Self::Error> {
        let username = context
            .cache
            .get_current_user()
            .map(|u| u.username.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        println!("✓ Logged into {}", username);
        Ok(())
    }

    async fn message(&self, context: Context, message: Message) -> Result<(), Self::Error> {
        self.command_handler
            .process_commands(context, message)
            .await
    }
}
