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
};

use super::Event;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("provider init failed")]
    ProviderInit(ProviderError),
    #[error("provider failed to get transaction")]
    ProviderGetTx(ProviderError),
}

/// Runs the TxCollector service, connecting to the execution node, then listening for new
/// transactions and sending them to the event channel.
/// # Errors
///
/// Returns a `searcher::Error::ProviderInit` if there is an error initializing a provider to
/// the endpoint.
pub(super) async fn run(wss_url: String, event_tx: Sender<Event>) -> Result<(), Error> {
    let provider = Provider::<Ws>::connect(format!("ws://{}", wss_url))
        .await
        .map_err(Error::ProviderInit)?;
    let stream = provider
        .subscribe_pending_txs()
        .await
        .map_err(Error::ProviderGetTx)?;
    let stream = stream.transactions_unordered(256);
    // get rid of errors
    let stream = stream.filter_map(|res| async move { res.ok() });
    // convert to searcher::Event
    let stream = stream.map(|tx| Event::NewTx(tx));

    let mut event_stream = Box::pin(stream);
    while let Some(event) = event_stream.next().await {
        match event_tx.send(event.clone()).await {
            Ok(()) => info!(?event, "NewTx was read from execution node"),
            Err(e) => error!(?e, "sending NewTx event failed",),
        }
    }
    Ok(())
}
