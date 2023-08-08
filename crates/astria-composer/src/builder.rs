use astria_sequencer::transaction::Signed as SequencerTxSigned;
use color_eyre::eyre::Context;
use sequencer_client::SequencerClientExt;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::ds::SequencerClient;
use color_eyre::eyre;

pub(crate) struct Builder {
    seq_client: SequencerClient,
    seq_tx_recv_channel: UnboundedReceiver<SequencerTxSigned>,
}

impl Builder {
    pub(crate) async fn new(
        seq_url: &str,
        seq_tx_recv_channel: UnboundedReceiver<SequencerTxSigned>,
    ) -> Result<Self, eyre::Error> {
        Ok(Self {
            seq_client: SequencerClient::new(seq_url)
                .wrap_err("Failed to initialize Sequencer Client")?,
            seq_tx_recv_channel,
        })
    }

    // FIXME: this shouldn't error on every failed submission
    pub(crate) async fn start(&mut self) ->  Result<(), eyre::Error>{
        while let Some(tx) = self.seq_tx_recv_channel.recv().await {
            self.seq_client
                .inner
                .submit_transaction_sync(tx)
                .await
                .wrap_err("failed to submit tx {tx} to sequencer")?;
        }

        Ok(())
    }
}
