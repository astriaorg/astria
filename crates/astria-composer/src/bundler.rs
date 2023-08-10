use crate::{
    ds::RollupTxExt,
    strategy::{NoStrategy, Strategy},
};
use color_eyre::eyre::{self, Context};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use tokio::sync::mpsc as tokio_mpsc;

/// The Bundler module bundles received transactions based on the various
/// MEV strategies that are implemented.
/// It then forwards the bundles to the Executor
pub(crate) struct Bundler {
    collector_receiver: UnboundedReceiver<RollupTxExt>,
    executor_sender: UnboundedSender<Vec<RollupTxExt>>,
}

impl Bundler {
    pub(crate) fn new(
        collector_receiver: UnboundedReceiver<RollupTxExt>,
    ) -> (Self, UnboundedReceiver<Vec<RollupTxExt>>) {
        let (stream, sink) = tokio_mpsc::unbounded_channel::<Vec<RollupTxExt>>();
        let new_strategy = Self {
            collector_receiver,
            executor_sender: stream,
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
            let sender = self.executor_sender.clone();

            // NOTE: This should be replaced with more sophisticated logic on how a list of transactions
            //       on which MEV strategies are executed is collected
            let pre_mev_bundle = vec![rollup_tx];

            // MEV on collated gathered bundles should be independent of other bundles and thus can their
            // own async task
            tokio::task::spawn(async move {
                let post_mev_bundle = Self::bundle_transactions(pre_mev_bundle).await.unwrap();
                sender
                    .send(post_mev_bundle)
                    .wrap_err("Failed to forward signed sequencer tx from strategy to builder")
                    .unwrap();
            });
        }

        Ok(())
    }
}
