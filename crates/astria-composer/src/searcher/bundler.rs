use tokio::sync::broadcast::{
    error::{
        RecvError,
        SendError,
    },
    Receiver,
    Sender,
};
use tracing::{
    error,
    trace,
};

use super::{
    Action,
    Event,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("receiving event failed")]
    EventRecv(#[source] RecvError),
    #[error("sending action failed")]
    ActionSend(#[source] SendError<Action>),
}

pub struct Bundler();

impl Bundler {
    pub(super) fn new() -> Self {
        Self()
    }

    pub(super) async fn run(
        self,
        mut event_rx: Receiver<Event>,
        action_tx: Sender<Action>,
    ) -> Result<(), Error> {
        // grab event
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let action = Self::process_event(event);
                    match action_tx.send(action.clone()) {
                        Ok(_) => trace!(action=?action, "action sent"),
                        Err(e) => {
                            error!(error=?e, "sending action failed");
                            // todo!("kill the executor?");
                            return Err(Error::ActionSend(e));
                        }
                    }
                }
                Err(e) => {
                    error!(error=?e, "receiving event failed");
                    // todo!("kill the bundler?");
                    return Err(Error::EventRecv(e));
                }
            }
        }
    }

    fn process_event(event: Event) -> Action {
        match event {
            Event::NewTx(tx) => {
                // serialize and pack into sequencer tx
                // send action with sequencer tx
                Action::SendSequencerSecondaryTx
            }
        }
    }
}
