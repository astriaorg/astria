use async_trait::async_trait;
use color_eyre::eyre::{self, Context};
use ethers::providers::{JsonRpcClient, Provider as EthersProvider, PubsubClient, StreamExt};
use ethers::{providers::Middleware, types::Transaction as EthersTx};
use tokio::sync::mpsc as tokio_mpsc;

#[non_exhaustive]
pub enum RollupTx {
    EthersTx(EthersTx),
}

pub(crate) trait WireFormat {
    fn serialize(&self) -> Box<[u8]>;
}

impl WireFormat for EthersTx {
    fn serialize(&self) -> Box<[u8]> {
        self.rlp().to_vec().into_boxed_slice()
    }
}

pub(crate) type RollupChainId = String;
pub type RollupTxExt = (RollupTx, RollupChainId);

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
