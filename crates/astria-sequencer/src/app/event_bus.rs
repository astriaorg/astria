use std::sync::Arc;

use astria_core::sequencerblock::v1::SequencerBlock;
use astria_eyre::eyre::WrapErr as _;
use tendermint::abci::request::FinalizeBlock;
use tokio::sync::watch::{
    Receiver,
    Sender,
};

/// `EventReceiver` contains the receiver side of the events sent by the Sequencer App.
/// The listeners of the events can receive the latest value of the event by calling the
/// `receive` method.
#[derive(Clone)]
pub(crate) struct EventReceiver<T> {
    /// The receiver side of the watch which is read for the latest value of the event.
    /// We receive an Option over T because the sender side of the watch is designed to send
    /// Option values. This allows the sender value to send objects which do not have a `Default`
    /// implementation.
    inner: Receiver<Option<T>>,
}

impl<T> EventReceiver<T>
where
    T: Clone,
{
    /// Returns the latest value of the event, waiting for the value to change if it hasn't already.
    pub(crate) async fn receive(&mut self) -> astria_eyre::Result<T> {
        // We want to only receive the latest value through the receiver, so we wait for the
        // current value in the watch to change before we return it.
        self.inner
            .changed()
            .await
            .wrap_err("error waiting for latest event")?;
        Ok(self.inner.borrow_and_update().clone().expect(
            "receivers are only created through tokio::sync::watch::Sender::subscribe, which
            means that the initial value of None is marked as seen. Subsequent updates always
            set the value to Some(T). If this panic message is seen it means that either:
            1) the receiver in the watch::channel call was used instead of being dropped;
            2) the sender illegally set the value to None;
            3) the value in the channel was marged as changed/unseen; or
            4) the behavior of the tokio watch channel changed fundamentally.
            All of these would violate the invariants of the event bus.",
        ))
    }
}

/// `EventSender` contains the sender side of the events sent by the Sequencer App.
struct EventSender<T> {
    // A watch channel that is always starts unset. Once set, the `is_init` token is cancelled
    // and value in the channel will never be unset.
    inner: Sender<Option<T>>,
}

impl<T> EventSender<T> {
    /// Create a new sender for an event `T`.
    fn new() -> Self {
        // XXX: the receiver must dropped so that the only entrypoint to the subscription is
        // Sender::subscribe. This is to ensure that a value of Option<T> can be unwrapped
        // and the Option remains an implementation detail.
        let (sender, _) = tokio::sync::watch::channel(None);
        Self {
            inner: sender,
        }
    }

    /// Creates a receiver for events `T`.
    fn subscribe(&self) -> EventReceiver<T> {
        EventReceiver {
            inner: self.inner.subscribe(),
        }
    }

    /// Sends the event to all subscribers.
    fn send(&self, event: T) {
        self.inner.send_replace(Some(event));
    }
}

/// A subscription to the event bus.
///
/// Allows subscribing to specific events like [`Self::process_proposal_blocks`]
/// and [`Self::finalized_blocks`].
pub(crate) struct EventBusSubscription {
    process_proposal_blocks: EventReceiver<Arc<SequencerBlock>>,
    finalized_blocks: EventReceiver<Arc<FinalizeBlock>>,
}

impl EventBusSubscription {
    /// Receive sequencer blocks after the process proposal phase.
    ///
    /// The returned [`EventReceiver`] will always provide the next
    /// event and ignore the latest one.
    pub(crate) fn process_proposal_blocks(&self) -> EventReceiver<Arc<SequencerBlock>> {
        let mut receiver = self.process_proposal_blocks.clone();
        receiver.inner.mark_unchanged();
        receiver
    }

    /// Receive finalized blocks.
    ///
    /// The returned [`EventReceiver`] will always provide the next
    /// event and ignore the latest one.
    pub(crate) fn finalized_blocks(&self) -> EventReceiver<Arc<FinalizeBlock>> {
        let mut receiver = self.finalized_blocks.clone();
        receiver.inner.mark_unchanged();
        receiver
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
    /// Instantiates a new event bus.
    pub(super) fn new() -> Self {
        let process_proposal_block_sender = EventSender::new();
        let finalized_block_sender = EventSender::new();

        Self {
            process_proposal_block_sender,
            finalized_block_sender,
        }
    }

    /// Subscribe to the event bus.
    pub(super) fn subscribe(&self) -> EventBusSubscription {
        EventBusSubscription {
            process_proposal_blocks: self.process_proposal_block_sender.subscribe(),
            finalized_blocks: self.finalized_block_sender.subscribe(),
        }
    }

    /// Sends a process proposal block event over the event bus.
    pub(super) fn send_process_proposal_block(&self, sequencer_block: Arc<SequencerBlock>) {
        self.process_proposal_block_sender.send(sequencer_block);
    }

    /// Sends a finalized block event over the event bus.
    pub(super) fn send_finalized_block(&self, sequencer_block_commit: Arc<FinalizeBlock>) {
        self.finalized_block_sender.send(sequencer_block_commit);
    }
}
