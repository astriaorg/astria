use tokio::sync::mpsc;

use super::Order;

pub(super) struct Request {
    pub(super) order: Order,
    pub(super) to_requester: tokio::sync::oneshot::Sender<Response>,
}

// TODO: Populate this with interesting information about order cancellation
// or order acceptance into the orderpool.
pub(crate) struct Response;

pub(super) fn new() -> (Sender, Receiver) {
    let (tx, rx) = mpsc::channel(32);
    (
        Sender {
            inner: tx,
        },
        Receiver {
            inner: rx,
        },
    )
}

pub(super) struct Receiver {
    inner: mpsc::Receiver<Request>,
}

impl Receiver {
    pub(super) async fn recv(&mut self) -> Option<Request> {
        self.inner.recv().await
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SendError {
    #[error("no available capacity in order pool")]
    Full,
    #[error("order pool channel is closed")]
    Closed,
    #[error("order pool dropped the response channel before sending a response")]
    Dropped,
}

impl From<tokio::sync::mpsc::error::TrySendError<Request>> for SendError {
    fn from(value: tokio::sync::mpsc::error::TrySendError<Request>) -> Self {
        match value {
            mpsc::error::TrySendError::Full(_) => Self::Full,
            mpsc::error::TrySendError::Closed(_) => Self::Closed,
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for SendError {
    fn from(_value: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Dropped
    }
}

#[derive(Clone)]
pub(crate) struct Sender {
    inner: mpsc::Sender<Request>,
}

impl Sender {
    pub(crate) async fn send(&self, order: Order) -> Result<Response, SendError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.inner.try_send(Request {
            order,
            to_requester: tx,
        })?;
        let rsp = rx.await?;
        Ok(rsp)
    }
}
