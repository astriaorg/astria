use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use super::{rollup_tx::RollupTx, RollupChainId, RollupTxExt};
use crate::searcher::clients::EthClient;

/// This trait represents a node client that can stream data into
/// a collector
#[async_trait]
pub(crate) trait StreamingClient {
    type Error;
    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<UnboundedReceiver<RollupTxExt>, Self::Error>;
}

// All the Clients we currently support

use color_eyre::eyre::{self, Context};
use ethers::providers::{Middleware, StreamExt};

/// This is a generic client implementation for an Ethers-rs Provider that supports streams
#[async_trait]
impl<C> StreamingClient for ethers::providers::Provider<C>
where
    C: ethers::providers::JsonRpcClient + ethers::providers::PubsubClient,
{
    type Error = eyre::Error;

    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<UnboundedReceiver<RollupTxExt>, Self::Error> {
        let (stream, sink) = unbounded_channel();

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
#[async_trait]
impl StreamingClient for EthClient {
    type Error = eyre::Error;

    async fn start_stream(
        &self,
        chain_id: RollupChainId,
    ) -> Result<UnboundedReceiver<RollupTxExt>, Self::Error> {
        self.start_stream(chain_id).await
    }
}
