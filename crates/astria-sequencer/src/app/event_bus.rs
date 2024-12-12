use std::sync::Arc;

use astria_core::sequencerblock::v1::SequencerBlock;
use astria_eyre::eyre::WrapErr as _;
use tendermint::abci::request::FinalizeBlock;
use tokio::sync::watch::{
    Receiver,
    Sender,
};
use tokio_util::sync::CancellationToken;

/// `EventReceiver` contains the receiver side of the events sent by the Sequencer App.
/// The listeners of the events can receive the latest value of the event by calling the
/// `receive` method.
#[derive(Clone)]
pub(crate) struct EventReceiver<T> {
    // The receiver side of the watch which is read for the latest value of the event.
    // We receive an Option over T because the sender side of the watch is designed to send
    // Option values. This allows the sender value to send objects which do not have a `Default`
    // implementation.
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
    // Marks the current message in the receiver end of the watch as seen.
    // This is useful in situations where we want to ignore the current value of the watch
    // and wait for the next value.
    pub(crate) fn mark_latest_event_as_seen(&mut self) {
        self.receiver.mark_unchanged();
    }

    // Returns the latest value of the event, waiting for the value to change if it hasn't already.
    pub(crate) async fn receive(&mut self) -> astria_eyre::Result<T> {
        // This will get resolved on the first send through the sender side of the watch
        // i.e when the sender is initialized.
        self.is_init.cancelled().await;
        // We want to only receive the latest value through the receiver, so we wait for the
        // current value in the watch to change before we return it.
        self.receiver
            .changed()
            .await
            .wrap_err("error waiting for latest event")?;
        Ok(self.receiver.borrow_and_update().clone().expect(
            "events must be set after is_init is triggered; this means an invariant was violated",
        ))
    }
}

/// `EventSender` contains the sender side of the events sent by the Sequencer App.
/// At any given time, it sends the latest value of the event.
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

    // Returns a `EventReceiver` object that contains the receiver side of the watch which can be
    // used to receive the latest value of the event.
    fn subscribe(&self) -> EventReceiver<T> {
        EventReceiver {
            receiver: self.sender.subscribe(),
            is_init: self.is_init.clone(),
        }
    }

    // Sends the event to all the subscribers.
    fn send(&self, event: T) {
        self.sender.send_replace(Some(event));
        // after sending the first value, we resolve the is_init token to signal that the sender
        // side of the watch is initialized. The receiver side can now start receiving valid
        // values.
        self.is_init.cancel();
    }
}

/// `EventBusSubscription` contains [`EventReceiver`] of various events that can be subscribed.
/// It can be cloned by various components in the sequencer app to receive events.
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

/// The Sequencer `EventBus` is used to send and receive events between different components of the
/// sequencer. Components of Sequencer can subscribe to the `EventBus` via the `subscribe` method
/// which returns a [`EventBusSubscription`] objects that contains receivers of various events which
/// are of type [`EventReceiver`].
///
/// The `EventBus` is implemented using [`tokio::sync::watch`] which allows for multiple receivers
/// to receive the event at any given time.
pub(super) struct EventBus {
    // Sends a process proposal block event to the subscribers. The event is sent in the form of a
    // sequencer block which is created during the process proposal block phase.
    process_proposal_block_sender: EventSender<Arc<SequencerBlock>>,
    // Sends a finalized block event to the subscribers. The event is sent in the form of the
    // finalize block abci request.
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

    // Returns a `EventBusSubscription` object that contains receivers of various events that can
    // be subscribed to.
    pub(crate) fn subscribe(&self) -> EventBusSubscription {
        EventBusSubscription {
            process_proposal_blocks: self.process_proposal_block_sender.subscribe(),
            finalized_blocks: self.finalized_block_sender.subscribe(),
        }
    }

    // Sends a process proposal block event to the subscribers.
    pub(super) fn send_process_proposal_block(&self, sequencer_block: Arc<SequencerBlock>) {
        self.process_proposal_block_sender.send(sequencer_block);
    }

    // Sends a finalized block event to the subscribers.
    pub(super) fn send_finalized_block(&self, sequencer_block_commit: Arc<FinalizeBlock>) {
        self.finalized_block_sender.send(sequencer_block_commit);
    }
}
