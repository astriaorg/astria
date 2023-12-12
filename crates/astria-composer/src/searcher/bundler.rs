use std::time::Duration;

use color_eyre::eyre;
use proto::native::sequencer::v1alpha1::{
    Action,
    SequenceAction,
};
use tokio::{
    select,
    sync::{
        mpsc,
        watch,
    },
    time,
};
use tracing::{
    debug,
    error,
    warn,
};

use super::collector::RollupTransaction;

// TODO: should get this from the ChainId type
const CHAIN_ID_LEN: usize = 32;

fn to_sequence_action(ru_transaction: RollupTransaction) -> SequenceAction {}

/// The status of this `Bundler` instance.
// TODO: should this report current buffer transactions, chain_ids?
#[derive(Debug)]
pub(super) struct Status {
    buffer_size: usize,
}

impl Status {
    fn new() -> Self {
        Self {
            buffer_size: 0,
        }
    }

    pub(super) fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

/// Bundles `RollupTransaction`s collected from rollup nodes into `Vec<Action>`s to be
/// submitted to the sequencer. Bundler is a sub-actor in the Searcher module that interfaces
///
/// `Bundler` is a sub-actor in the Searcher module that interfaces between individual `Collector`s
/// and an `Executor`. This implementation simply buffers arriving `RollupTransaction`s in a FIFO
/// order until it reaches the maximum size for an `Vec<Action>`. In order to ensure timely
/// arrival of `RollupTransaction`s to their destination rollup, the `Bundler` will flush the buffer
/// and send the `Vec<Action>` to the `Executor` at least once every block, configured by
pub(super) struct Bundler {
    // The status of this `Bundler` instance.
    status: watch::Sender<Status>,
    // Max amount of bytes to fit in a bundle.
    max_bytes_size: usize,
    // The channel on which the bundler receives new `RollupTransaction`s from the `Collector`.
    rollup_transactions_rx: mpsc::Receiver<RollupTransaction>,
    // The channel on which the bundler sends new `Vec<Action>`s to the `Executor`.
    bundles_tx: mpsc::Sender<Vec<Action>>,
    // Duration to wait for backpressure before dropping the transaction as stale
    backpressure_timeout: u64,
    // Channel to receive block timer ticks from the executor
    block_timer: 
}

impl Bundler {
    /// Initializes a new bundler instance
    pub fn new(
        rollup_transactions_rx: mpsc::Receiver<RollupTransaction>,
        sequencer_transactions_tx: mpsc::Sender<Vec<Action>>,
        max_bytes_size: usize,
        backpressure_timeout: u64,
    ) -> Self {
        let (status, _) = watch::channel(Status::new());
        Self {
            status,
            max_bytes_size,
            rollup_transactions_rx,
            bundles_tx: sequencer_transactions_tx,
            backpressure_timeout,
        }
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    async fn flush_to_executor(&mut self, bundle: Vec<Action>) -> eyre::Result<()> {
        // TODO: this is where backpressure resulting from the executor's transaction
        // submission would happen. need to properly report this here
        if let Err(e) = self
            .bundles_tx
            // try_send_timeout instead
            .send_timeout(bundle, Duration::from_millis(self.backpressure_timeout))
            .await
        {
            error!(
                    error.message = %e,
                    error.cause_chain=?e,
                    "failed to forward bundle to executor due to backpressure timeoout"
            )
        }
        Ok(())
    }

    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let mut curr_bundle = vec![];
        let mut curr_bytes = 0;

        loop {
            select! {
                Some(ru_transaction) = self.rollup_transactions_rx.recv() => {
                    // convert to seq action
                    let seq_action = SequenceAction {
                        chain_id: ru_transaction.chain_id,
                        data: ru_transaction.inner.rlp().to_vec(),
                    };

                    // check seq action length
                    if seq_action.data.len() + CHAIN_ID_LEN > self.max_bytes_size {
                        warn!(
                            transaction.chain_id = ru_transaction.chain_id.to_string(),
                            transaction.hash = ru_transaction.inner.hash.to_string(),
                            "failed to bundle rollup transaction: transaction is too large. Transaction is dropped."
                        );
                        continue;
                    }

                    // if buffer doesn't have space for tx, flush it
                    if curr_bytes + CHAIN_ID_LEN + seq_action.data.len() > self.max_bytes_size {
                        debug!(
                            "bundler's buffer is full, flushing all buffered actions to the executor"
                        );
                        // TODO: move async out of body of select?
                        // change to try_send
                        self.flush_to_executor(curr_bundle).await?;
                        curr_bundle = vec![];
                    }

                    // otherwise, add to buffer
                    // TODO: use ru_transaction's span to map ru_tx to bundle
                    debug!(
                        transaction.chain_id = seq_action.chain_id.to_string(),
                        bytes = curr_bytes,
                        "bundled rollup transaction",
                    );
                    curr_bytes += CHAIN_ID_LEN + seq_action.data.len();
                    curr_bundle.push(Action::Sequence(seq_action));
                }

                // receive bundle request signal from executor
                _ = block_timer.tick() => {
                    // receive oneshot from executor
                    // flush bundle to the oneshot
                    debug!("bundler's block timer tick, flushing actions to the executor to ensure timely arrival of rollup transactions");
                    self.flush_to_executor(curr_bundle).await?;
                    curr_bundle = vec![];
                }
            }
        }
    }
}
