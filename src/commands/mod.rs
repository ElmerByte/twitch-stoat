mod addstream;
mod helpstream;
mod liststreams;
mod removestream;

use crate::{error::Error, state::State};
use stoat::async_trait;
use stoat::commands::{
    Command, CommandEventHandler, CommandHandler as StoatCommandHandler, Context as CommandContext,
};

pub use addstream::addstream;
pub use helpstream::helpstream;
pub use liststreams::liststreams;
pub use removestream::removestream;

pub type CmdCtx = CommandContext<Error, State>;
pub type CommandHandler = StoatCommandHandler<Commands>;

#[derive(Clone)]
pub struct Commands;

#[async_trait]
impl CommandEventHandler for Commands {
    type State = State;
    type Error = Error;

    async fn get_prefix(&self, _ctx: CmdCtx) -> Result<Vec<String>, Error> {
        Ok(vec!["!".to_string()])
    }
}

pub fn create_handler(state: State) -> CommandHandler {
    StoatCommandHandler::new(Commands, state).register(vec![
        Command::new("addstream", addstream).description("Add a Twitch channel to monitor"),
        Command::new("removestream", removestream).description("Remove a monitored channel"),
        Command::new("liststreams", liststreams).description("List monitored channels"),
        Command::new("helpstream", helpstream).description("Show available commands"),
    ])
}
