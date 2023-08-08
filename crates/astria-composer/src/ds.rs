use async_trait::async_trait;
use color_eyre::eyre::{self, Context};
use ethers::providers::{JsonRpcClient, Provider as EthersProvider, PubsubClient, StreamExt};
use ethers::{providers::Middleware, types::Transaction as EthersTx};
use std::time::Duration;
use tendermint::abci;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::instrument;

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

/// A thin wrapper around [`sequencer_client::Client`] to add timeouts.
///
/// Currently only provides a timeout for `abci_info`.
#[derive(Clone)]
pub(crate) struct SequencerClient {
    pub(crate) inner: sequencer_client::HttpClient,
}

impl SequencerClient {
    #[instrument]
    pub(crate) fn new(url: &str) -> eyre::Result<Self> {
        let inner = sequencer_client::HttpClient::new(url)
            .wrap_err("failed to construct sequencer client")?;
        Ok(Self { inner })
    }

    /// Wrapper around [`Client::abci_info`] with a 1s timeout.
    pub(crate) async fn abci_info(self) -> eyre::Result<abci::response::Info> {
        use sequencer_client::Client as _;
        tokio::time::timeout(Duration::from_secs(1), self.inner.abci_info())
            .await
            .wrap_err("request timed out")?
            .wrap_err("RPC returned with error")
    }
}
