use std::sync::Arc;

use astria_core::sequencerblock::v1::{
    optimistic_block::SequencerBlockCommit,
    SequencerBlock,
};
use tokio::sync::watch::{
    Receiver,
    Sender,
};
use tokio_util::sync::CancellationToken;
use tracing::error;

#[derive(Clone)]
pub(crate) struct EventReceiver<T> {
    receiver: Receiver<Option<T>>,
    is_init: CancellationToken,
}

impl<T> EventReceiver<T>
where
    T: Clone,
{
    pub(crate) async fn receive(&mut self) -> astria_eyre::Result<T> {
        self.is_init.cancelled().await;
        self.receiver.changed().await?;
        Ok(self
            .receiver
            .borrow_and_update()
            .clone()
            .expect("unexpected value passed in event receiver"))
    }
}

#[derive(Clone)]
pub(crate) struct EventSender<T> {
    sender: Sender<Option<T>>,
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
        if self.sender.receiver_count() > 0 {
            if let Err(e) = self.sender.send(Some(event)) {
                error!(error = %e, "failed to send event");
            }
            self.is_init.cancel();
        }
    }
}

#[derive(Clone)]
pub(crate) struct EventBus {
    process_proposal_block_sender: EventSender<Arc<SequencerBlock>>,
    finalize_block_sender: EventSender<Arc<SequencerBlockCommit>>,
}

impl crate::app::EventBus {
    pub(crate) fn new() -> Self {
        let process_proposal_block_sender = EventSender::new();
        let finalize_block_sender = EventSender::new();

        Self {
            process_proposal_block_sender,
            finalize_block_sender,
        }
    }

    pub(crate) fn subscribe_process_proposal_blocks(&self) -> EventReceiver<Arc<SequencerBlock>> {
        self.process_proposal_block_sender.subscribe()
    }

    pub(crate) fn subscribe_finalize_blocks(&self) -> EventReceiver<Arc<SequencerBlockCommit>> {
        self.finalize_block_sender.subscribe()
    }

    pub(crate) fn send_process_proposal_block(&self, sequencer_block: SequencerBlock) {
        self.process_proposal_block_sender
            .send(Arc::new(sequencer_block));
    }

    pub(crate) fn send_finalize_block(&self, sequencer_block_commit: SequencerBlockCommit) {
        self.finalize_block_sender
            .send(Arc::new(sequencer_block_commit));
    }
}
