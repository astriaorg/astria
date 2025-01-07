use std::sync::Arc;

use astria_core::sequencerblock::v1::SequencerBlock;
use tendermint::abci::request::FinalizeBlock;
use tokio::sync::watch::{
    Receiver,
    Sender,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(crate) struct EventReceiver<T> {
    // The receiver side of the watch which is read for the latest value of the event.
    receiver: Receiver<Option<T>>,
    // A token that is resolved when the sender side of the watch is initialized.
    // It is used to wait for the sender side of the watch to be initialized before the
    // receiver side can start receiving valid values.
    is_init: CancellationToken,
}

impl<T> EventReceiver<T>
where
    T: Clone,
{
    pub(crate) async fn receive(&mut self) -> astria_eyre::Result<T> {
        // this will get resolved on the first send through the sender side of the watch
        // i.e when the sender is initialized.
        self.is_init.cancelled().await;
        // we want to only receive the latest value through the receiver, so we wait for the
        // current value in the watch to change before we return it.
        self.receiver.changed().await?;
        Ok(self.receiver.borrow_and_update().clone().expect(
            "values must be set after is_init is triggered; this means an invariant was violated",
        ))
    }
}

#[derive(Clone)]
struct EventSender<T> {
    // The sender side of the watch which is used to send the latest value of the event.
    // We use an Option here to allow for the sender to be initialized with a None value
    // which allows the type to not have a Default value.
    sender: Sender<Option<T>>,
    // A token that is resolved when the sender side of the watch is initialized. It is used to
    // wait for the sender side of the watch to be initialized before the receiver side can start
    // receiving valid values.
    is_init: CancellationToken,
}

impl<T> EventSender<T> {
    fn new() -> Self {
        let (sender, _) = tokio::sync::watch::channel(None);
        Self {
            sender,
            is_init: CancellationToken::new(),
        }
    }

    fn subscribe(&self) -> EventReceiver<T> {
        EventReceiver {
            receiver: self.sender.subscribe(),
            is_init: self.is_init.clone(),
        }
    }

    fn send(&self, event: T) {
        self.sender.send_replace(Some(event));
        // after sending the first value, we resolve the is_init token to signal that the sender
        // side of the watch is initialized. The receiver side can now start receiving valid
        // values.
        self.is_init.cancel();
    }
}

#[derive(Clone)]
pub(crate) struct EventBusSubscription {
    process_proposal_blocks: EventReceiver<Arc<SequencerBlock>>,
    finalized_blocks: EventReceiver<Arc<FinalizeBlock>>,
}

impl EventBusSubscription {
    pub(crate) fn process_proposal_blocks(&mut self) -> EventReceiver<Arc<SequencerBlock>> {
        self.process_proposal_blocks.clone()
    }

    pub(crate) fn finalized_blocks(&mut self) -> EventReceiver<Arc<FinalizeBlock>> {
        self.finalized_blocks.clone()
    }
}

pub(super) struct EventBus {
    process_proposal_block_sender: EventSender<Arc<SequencerBlock>>,
    finalized_block_sender: EventSender<Arc<FinalizeBlock>>,
}

impl EventBus {
    pub(super) fn new() -> Self {
        let process_proposal_block_sender = EventSender::new();
        let finalized_block_sender = EventSender::new();

        Self {
            process_proposal_block_sender,
            finalized_block_sender,
        }
    }

    pub(crate) fn subscribe(&self) -> EventBusSubscription {
        EventBusSubscription {
            process_proposal_blocks: self.process_proposal_block_sender.subscribe(),
            finalized_blocks: self.finalized_block_sender.subscribe(),
        }
    }

    pub(super) fn send_process_proposal_block(&self, sequencer_block: Arc<SequencerBlock>) {
        self.process_proposal_block_sender.send(sequencer_block);
    }

    pub(super) fn send_finalized_block(&self, sequencer_block_commit: Arc<FinalizeBlock>) {
        self.finalized_block_sender.send(sequencer_block_commit);
    }
}
