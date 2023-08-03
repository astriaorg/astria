use ethers::providers::{
    JsonRpcClient,
    Middleware,
    Provider,
    ProviderError,
    PubsubClient,
    StreamExt,
    Ws,
};
use tokio::sync::broadcast::{
    error::SendError,
    Sender,
};
use tracing::{
    error,
    trace,
};

use super::Event;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("provider init failed")]
    ProviderInit(#[source] ProviderError),
    #[error("provider failed to get transactions subscription")]
    ProviderSubscriptionError(#[source] ProviderError),
    #[error("sending event failed")]
    EventSend(#[source] SendError<Event>),
}

#[derive(Debug)]
pub struct TxCollector<P>
where
    P: PubsubClient + JsonRpcClient + Clone,
{
    provider: Provider<P>,
}

impl TxCollector<Ws> {
    /// Constructs a new TxCollector service from config, connecting the ethers Provider to the
    /// execution node.
    pub(super) fn new(provider: Provider<Ws>) -> Self {
        Self {
            provider,
        }
    }

    /// Runs the TxCollector service, listening for new transactions from the execution node and
    /// sending them to the event channel.
    pub(super) async fn run(self, event_tx: Sender<Event>) -> Result<(), Error> {
        // get stream of pending txs from execution node
        let stream = self
            .provider
            .subscribe_full_pending_txs()
            .await
            .map_err(Error::ProviderSubscriptionError)?;
        // convert to searcher::Event
        let stream = stream.map(|tx| Event::NewRollupTx(tx));

        // pass txs to event channel
        let mut event_stream = Box::pin(stream);
        while let Some(event) = event_stream.next().await {
            trace!(?event, "NewRollupTx was read from execution node");
            match event_tx.send(event) {
                Ok(_) => {}
                Err(e) => {
                    error!(error=?e, "sending NewTx event failed");
                    todo!("kill the tx collector")
                }
            }
        }
        Ok(())
    }
}
