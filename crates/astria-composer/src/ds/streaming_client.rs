use async_trait::async_trait;
use ethers::providers::{
    JsonRpcClient, Middleware, Provider as EthersProvider, PubsubClient, StreamExt,
};
use tokio::sync::mpsc as tokio_mpsc;
use color_eyre::eyre::{self, Context};

use super::{RollupChainId, RollupTx, RollupTxExt};

#[async_trait]
pub(crate) trait StreamingClient {
    type Error;
    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<tokio_mpsc::UnboundedReceiver<RollupTxExt>, Self::Error>;
}

#[async_trait]
impl<C> StreamingClient for EthersProvider<C>
where
    C: JsonRpcClient + PubsubClient,
{
    type Error = eyre::Error;

    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<tokio_mpsc::UnboundedReceiver<RollupTxExt>, Self::Error> {
        let (stream, sink) = tokio_mpsc::unbounded_channel();

        let mut pending_tx_stream = self
            .subscribe_full_pending_txs()
            .await
            .wrap_err("couldn't subscribe to eth pending tx stream over ws")?;

        while let Some(tx) = pending_tx_stream.next().await {
            let chain_id_clone = chain_id.clone();
            stream
                .send((RollupTx::EthersTx(tx), chain_id_clone))
                .wrap_err("could not send tx to stream")?;
        }

        Ok(sink)
    }
}
