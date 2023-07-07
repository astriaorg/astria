use tokio::sync::broadcast::{
    Receiver,
    Sender,
};
use tracing::error;

use super::{
    Action,
    Event,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub struct Executor();

impl Executor {
    pub(super) fn new() -> Self {
        Self()
    }

    pub(super) async fn run(
        self,
        event_rx: Receiver<Event>,
        _action_tx: Sender<Action>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
