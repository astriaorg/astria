use ethers::providers::{
    Middleware,
    Provider,
    ProviderError,
    StreamExt,
    Ws,
};
use tokio::sync::mpsc::Sender;
use tracing::{
    error,
    info,
    trace,
};

use super::Event;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("provider init failed")]
    ProviderInit(#[source] ProviderError),
    #[error("provider failed to get transactions subscription")]
    ProviderSubscriptionError(#[source] ProviderError),
}

pub struct TxCollector {
    provider: Provider<Ws>,
}

impl TxCollector {
    /// Constructs a new TxCollector service from config, connecting the ethers Provider to the
    /// execution node.
    ///
    /// # Errors
    ///
    /// Returns a `searcher::Error::ProviderInit` if there is an error initializing a provider to
    /// the endpoint.
    pub(super) async fn new(ws_url: &str) -> Result<Self, Error> {
        let provider = Provider::<Ws>::connect(format!("ws://{}", ws_url))
            .await
            .map_err(Error::ProviderInit)?;
        info!(?ws_url, "connected to execution node");
        Ok(Self {
            provider,
        })
    }

    /// Runs the TxCollector service, listening for new transactions from the execution node and
    /// sending them to the event channel.
    /// # Errors
    ///
    /// Returns a `searcher::Error::ProviderGetTx` if there is an error getting transactions from
    /// the node.
    pub(super) async fn run(self, event_tx: Sender<Event>) -> Result<(), Error> {
        // get stream of pending txs from execution node
        let stream = self
            .provider
            .subscribe_pending_txs()
            .await
            .map_err(Error::ProviderSubscriptionError)?;
        let stream = stream.transactions_unordered(256);
        // get rid of errors
        let stream = stream.filter_map(|res| async move { res.ok() });
        // convert to searcher::Event
        let stream = stream.map(|tx| Event::NewTx(tx));

        // pass txs to event channel
        let mut event_stream = Box::pin(stream);
        while let Some(event) = event_stream.next().await {
            match event_tx.send(event.clone()).await {
                Ok(()) => trace!(?event, "NewTx was read from execution node"),
                Err(e) => {
                    error!(error=?e, "sending NewTx event failed");
                    todo!("kill the tx collector")
                }
            }
        }
        Ok(())
    }
}
