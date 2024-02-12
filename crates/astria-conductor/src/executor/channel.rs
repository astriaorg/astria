//! An mpsc channel bounded by an externally driven semaphore.

use std::sync::Arc;

use sequencer_client::SequencerBlock;
use tokio::sync::{
    mpsc::{
        error::SendError as TokioSendError,
        unbounded_channel,
        UnboundedReceiver,
        UnboundedSender,
    },
    AcquireError,
    Semaphore,
    TryAcquireError,
};

/// Creates an mpsc channel for sending soft blocks between asynchronous task.
///
/// The initial bound of the channel is 0 and the receiver is expected to add
/// capacity to the channel.
pub(super) fn soft_block_channel() -> (Sender, Receiver) {
    let cap = 0;
    let sem = Arc::new(Semaphore::new(0));
    let (tx, rx) = unbounded_channel();
    let sender = Sender {
        sem: sem.clone(),
        chan: tx,
    };
    let receiver = Receiver {
        cap,
        sem,
        chan: rx,
    };
    (sender, receiver)
}

#[derive(Debug, thiserror::Error)]
#[error("the channel is closed")]
pub(crate) struct SendError;

impl From<AcquireError> for SendError {
    fn from(_: AcquireError) -> Self {
        Self
    }
}

impl From<TokioSendError<SequencerBlock>> for SendError {
    fn from(_: TokioSendError<SequencerBlock>) -> Self {
        Self
    }
}

// allow: this is mimicking tokio's `SendError` that returns the stack-allocated object.
#[allow(clippy::result_large_err)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum TrySendError {
    #[error("the channel is closed")]
    Closed(SequencerBlock),
    #[error("no permits available")]
    NoPermits(SequencerBlock),
}

impl TrySendError {
    fn from_semaphore(err: &TryAcquireError, block: SequencerBlock) -> Self {
        match err {
            tokio::sync::TryAcquireError::Closed => Self::Closed(block),
            tokio::sync::TryAcquireError::NoPermits => Self::NoPermits(block),
        }
    }
}

impl From<TokioSendError<SequencerBlock>> for TrySendError {
    fn from(err: TokioSendError<SequencerBlock>) -> Self {
        Self::Closed(err.0)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Sender {
    sem: Arc<Semaphore>,
    chan: UnboundedSender<SequencerBlock>,
}

impl Sender {
    /// Sends a block, waiting until the channel has permits.
    ///
    /// Returns an error if the channel is closed.
    pub(super) async fn send(&self, block: SequencerBlock) -> Result<(), SendError> {
        let permit = self.sem.acquire().await?;
        permit.forget();
        self.chan.send(block)?;
        Ok(())
    }

    /// Attempts to send a block without blocking.
    ///
    /// Returns an error if the channel is out of permits or if it has been closed.
    // allow: this is mimicking tokio's `SendError` that returns the stack-allocated object.
    #[allow(clippy::result_large_err)]
    pub(super) fn try_send(&self, block: SequencerBlock) -> Result<(), TrySendError> {
        let permit = match self.sem.try_acquire() {
            Ok(permit) => permit,
            Err(err) => return Err(TrySendError::from_semaphore(&err, block)),
        };
        permit.forget();
        self.chan.send(block)?;
        Ok(())
    }
}

pub(super) struct Receiver {
    cap: usize,
    sem: Arc<Semaphore>,
    chan: UnboundedReceiver<SequencerBlock>,
}

impl Receiver {
    /// Sets the channel's capacity to `cap`.
    ///
    /// `cap` will be the maximum number of blocks that can be sent
    /// over the channel before new permits are added with `[SoftBlockReceiver::add_permits]`.
    pub(super) fn set_capacity(&mut self, cap: usize) {
        self.cap = cap;
    }

    /// Adds up to `capacity` number of permits to the channel.
    ///
    /// `capacity` is previously set by [`SoftBlockReceiver::set_capacity`]
    /// or zero by default.
    pub(super) fn fill_permits(&self) {
        let additional = self.cap.saturating_sub(self.sem.available_permits());
        self.sem.add_permits(additional);
    }

    /// Receives a block over the channel.
    pub(super) async fn recv(&mut self) -> Option<SequencerBlock> {
        self.chan.recv().await
    }
}
