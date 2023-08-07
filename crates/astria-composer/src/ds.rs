use async_trait::async_trait;
use color_eyre::eyre::{self, Context};
use ethers::providers::{Provider as EthersProvider, StreamExt};
use ethers::{
    providers::{Middleware, Ws},
    types::Transaction as EthersTx,
};
use tokio::sync::mpsc as tokio_mpsc;

#[non_exhaustive]
pub(crate) enum RollupTx {
    EthersTx(EthersTx),
}

#[async_trait]
pub(crate) trait StreamingClient {
    type Error;
    async fn start_stream(&self) -> Result<tokio_mpsc::UnboundedReceiver<RollupTx>, Self::Error>;
}

#[async_trait]
impl StreamingClient for EthersProvider<Ws> {
    type Error = eyre::Error;

    async fn start_stream(&self) -> Result<tokio_mpsc::UnboundedReceiver<RollupTx>, Self::Error> {
        let (stream, sink) = tokio_mpsc::unbounded_channel();

        let mut pending_tx_stream = self
            .subscribe_full_pending_txs()
            .await
            .wrap_err("couldn't subscribe to eth pending tx stream over ws")?;

        while let Some(tx) = pending_tx_stream.next().await {
            stream
                .send(RollupTx::EthersTx(tx))
                .wrap_err("could not send tx to stream")?;
        }

        Ok(sink)
    }
}
