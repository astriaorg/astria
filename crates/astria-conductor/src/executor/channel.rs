//! An mpsc channel bounded by an externally driven semaphore.
//!
//! While the main purpose of this channel is to send [`sequencer_client::SequencerBlock`]s
//! from a sequencer reader to the executor, the channel is generic over the values that are
//! being sent to better test its functionality.

use std::sync::{
    Arc,
    Weak,
};

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
pub(super) fn soft_block_channel<T>() -> (Sender<T>, Receiver<T>) {
    let cap = 0;
    let sem = Arc::new(Semaphore::new(0));
    let (tx, rx) = unbounded_channel();
    let sender = Sender {
        chan: tx,
        sem: Arc::downgrade(&sem),
    };
    let receiver = Receiver {
        cap,
        chan: rx,
        sem,
    };
    (sender, receiver)
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("the channel is closed")]
pub(crate) struct SendError;

impl From<AcquireError> for SendError {
    fn from(_: AcquireError) -> Self {
        Self
    }
}

impl<T> From<TokioSendError<T>> for SendError {
    fn from(_: TokioSendError<T>) -> Self {
        Self
    }
}

// allow: this is mimicking tokio's `SendError` that returns the stack-allocated object.
#[allow(clippy::result_large_err)]
#[derive(Debug, thiserror::Error, PartialEq)]
pub(crate) enum TrySendError<T> {
    #[error("the channel is closed")]
    Closed(T),
    #[error("no permits available")]
    NoPermits(T),
}

impl<T> TrySendError<T> {
    fn from_semaphore(err: &TryAcquireError, block: T) -> Self {
        match err {
            tokio::sync::TryAcquireError::Closed => Self::Closed(block),
            tokio::sync::TryAcquireError::NoPermits => Self::NoPermits(block),
        }
    }
}

impl<T> From<TokioSendError<T>> for TrySendError<T> {
    fn from(err: TokioSendError<T>) -> Self {
        Self::Closed(err.0)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Sender<T> {
    sem: Weak<Semaphore>,
    chan: UnboundedSender<T>,
}

impl<T> Sender<T> {
    /// Sends a block, waiting until the channel has permits.
    ///
    /// Returns an error if the channel is closed.
    pub(super) async fn send(&self, block: T) -> Result<(), SendError> {
        let sem = self.sem.upgrade().ok_or(SendError)?;
        let permit = sem.acquire().await?;
        permit.forget();
        self.chan.send(block)?;
        Ok(())
    }

    /// Attempts to send a block without blocking.
    ///
    /// Returns an error if the channel is out of permits or if it has been closed.
    // allow: this is mimicking tokio's `TrySendError` that returns the stack-allocated object.
    #[allow(clippy::result_large_err)]
    pub(super) fn try_send(&self, block: T) -> Result<(), TrySendError<T>> {
        let sem = match self.sem.upgrade() {
            None => return Err(TrySendError::Closed(block)),
            Some(sem) => sem,
        };
        let permit = match sem.try_acquire() {
            Err(err) => return Err(TrySendError::from_semaphore(&err, block)),
            Ok(permit) => permit,
        };
        permit.forget();
        self.chan.send(block)?;
        Ok(())
    }
}

pub(super) struct Receiver<T> {
    cap: usize,
    sem: Arc<Semaphore>,
    chan: UnboundedReceiver<T>,
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.sem.close();
    }
}

impl<T> Receiver<T> {
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
    pub(super) async fn recv(&mut self) -> Option<T> {
        self.chan.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::{
        soft_block_channel,
        SendError,
        TrySendError,
    };

    #[test]
    fn fresh_channel_has_no_capacity() {
        let (tx, _rx) = soft_block_channel::<()>();
        assert_eq!(
            tx.try_send(()).unwrap_err(),
            TrySendError::NoPermits(()),
            "a fresh channel starts without permits"
        );
    }

    #[test]
    fn permits_are_filled_to_capacity() {
        let cap = 2;
        let (tx, mut rx) = soft_block_channel::<()>();
        rx.set_capacity(cap);
        rx.fill_permits();
        for _ in 0..cap {
            tx.try_send(()).expect("the channel should have capacity");
        }
        assert_eq!(
            tx.try_send(()).unwrap_err(),
            TrySendError::NoPermits(()),
            "a channel that has its permits used up should return with a NoPermits error until \
             refilled or closed",
        );
    }

    #[test]
    fn refilling_twice_has_no_effect() {
        let cap = 2;
        let (tx, mut rx) = soft_block_channel::<()>();
        rx.set_capacity(cap);
        rx.fill_permits();
        rx.fill_permits();
        for _ in 0..cap {
            tx.try_send(()).expect("the channel should have capacity");
        }
        assert_eq!(
            tx.try_send(()).unwrap_err(),
            TrySendError::NoPermits(()),
            "refilling twice in a row should result in the same number of permits"
        );
    }

    #[test]
    fn try_sending_to_dropped_receiver_returns_closed_error() {
        let (tx, rx) = soft_block_channel::<()>();
        std::mem::drop(rx);
        assert_eq!(
            tx.try_send(()).unwrap_err(),
            TrySendError::Closed(()),
            "a channel with a dropped receiver is considered closed",
        );
    }

    #[tokio::test]
    async fn async_sending_to_dropped_receiver_returns_closed_error() {
        let (tx, rx) = soft_block_channel::<()>();
        std::mem::drop(rx);
        assert_eq!(
            tx.send(()).await.unwrap_err(),
            SendError,
            "a channel with a dropped receiver is considered closed",
        );
    }

    #[tokio::test]
    #[should_panic(expected = "receiving with all senders dropped should return None")]
    async fn receiving_without_any_remaining_receivers_returns_none() {
        let (tx, mut rx) = soft_block_channel::<()>();
        std::mem::drop(tx);
        rx.recv()
            .await
            .expect("receiving with all senders dropped should return None");
    }
}
