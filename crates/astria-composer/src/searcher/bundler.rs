use tokio::sync::{
    mpsc,
    oneshot,
};

use super::{
    Action,
    Event,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub struct Bundler();

impl Bundler {
    pub(super) fn new() -> Self {
        Self()
    }
}

pub(super) async fn run(
    event_rx: mpsc::Receiver<Event>,
    action_tx: oneshot::Sender<Action>,
) -> Result<(), Error> {
    // grab collected tx

    // serialize and pack into sequencer tx
    // send action with sequencer tx
    Ok(())
}
