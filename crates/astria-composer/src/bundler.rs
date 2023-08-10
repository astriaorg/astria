use crate::{
    ds::RollupTxExt,
    strategy::{NoStrategy, Strategy},
};
use color_eyre::eyre::{self, Context};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use tokio::sync::mpsc as tokio_mpsc;

pub(crate) struct Bundler {
    collector_receiver: UnboundedReceiver<RollupTxExt>,
    sequencer_sender: UnboundedSender<Vec<RollupTxExt>>,
}

impl Bundler {
    pub(crate) fn new(
        collector_receiver: UnboundedReceiver<RollupTxExt>,
    ) -> (Self, UnboundedReceiver<Vec<RollupTxExt>>) {
        let (stream, sink) = tokio_mpsc::unbounded_channel::<Vec<RollupTxExt>>();
        let new_strategy = Self {
            collector_receiver,
            sequencer_sender: stream,
        };
        (new_strategy, sink)
    }

    async fn bundle_transactions(
        rollup_txs: Vec<RollupTxExt>,
    ) -> Result<Vec<RollupTxExt>, eyre::Error> {
        // NOTE: Replace this with arbitrary strategy logic
        let bundle = NoStrategy::execute(rollup_txs).await?;
        Ok(bundle)
    }

    pub(crate) async fn start(&mut self) -> Result<(), eyre::Error> {
        while let Some(rollup_tx) = self.collector_receiver.recv().await {
            let sender = self.sequencer_sender.clone();
            // Serialization and Signing of one bundle should not block the serialization and signing of other
            // bundles
            tokio::task::spawn(async move {
                let bundle = Self::bundle_transactions(vec![rollup_tx]).await.unwrap();

                sender
                    .send(bundle)
                    .wrap_err("Failed to forward signed sequencer tx from strategy to builder")
                    .unwrap();
            });
        }

        Ok(())
    }
}
