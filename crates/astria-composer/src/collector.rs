use crate::ds::{RollupChainId, RollupTx, RollupTxExt, StreamingClient};
use color_eyre::eyre::{self};
use tokio::sync::mpsc as tokio_mpsc;

pub(crate) struct Collector {
    digestion_channel: tokio_mpsc::UnboundedSender<RollupTxExt>,
}

impl Collector {
    pub(crate) fn new() -> (Self, tokio_mpsc::UnboundedReceiver<RollupTxExt>) {
        let (stream, sink) = tokio_mpsc::unbounded_channel::<RollupTxExt>();
        let new_collector = Collector {
            digestion_channel: stream,
        };

        (new_collector, sink)
    }

    pub(crate) async fn add_provider<P>(
        &self,
        pr: P,
        chain_id: RollupChainId,
    ) -> Result<(), eyre::Error>
    where
        P: StreamingClient<Error = eyre::Error>,
    {
        let sender_clone = self.digestion_channel.clone();
        let mut receiver = pr.start_stream(chain_id).await?;

        tokio::task::spawn(async move {
            while let Some(tx) = receiver.recv().await {
                sender_clone.send(tx).unwrap();
            }
        });

        Ok(())
    }
}
