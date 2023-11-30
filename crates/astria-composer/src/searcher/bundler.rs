use std::{
    collections::HashMap,
    time::Duration,
};

use color_eyre::eyre;
use ethers::types::Transaction;
use proto::native::sequencer::v1alpha1::{
    Action,
    SequenceAction,
};
use sequencer_types::ChainId;
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

/// Max bytes size of a sequencer transactionis set to 250KB.
pub(super) const MAX_BYTES_SIZE: usize = 250_000;
// TODO: should get this from the ChainId type
const CHAIN_ID_LEN: usize = 32;

#[derive(Debug, thiserror::Error)]
enum TransactionBufferError {
    #[error("rollup transaction is too large")]
    RollupTransacionTooLarge,
    #[error(
        "sequencer transaction will be too large with the addition of this rollup transaction"
    )]
    BufferFull,
}

/// Buffer for building up a set of rollup transactions to turn into an `UnsignedTransaction` and
/// submit to the executor
struct TransactionBuffer {
    pub actions: Vec<Action>,
    pub bytes_sz: usize,
    max_bytes_size: usize,
}

impl TransactionBuffer {
    pub fn new(max_bytes_size: usize) -> Self {
        Self {
            actions: vec![],
            bytes_sz: 0,
            max_bytes_size,
        }
    }

    pub fn push(
        &mut self,
        ru_transaction: RollupTransaction,
    ) -> Result<(), TransactionBufferError> {
        let transaction_bytes = ru_transaction.inner.rlp().to_vec();
        let transaction_bytes_size = transaction_bytes.len();
        if transaction_bytes_size > self.max_bytes_size {
            return Err(TransactionBufferError::RollupTransacionTooLarge);
        }

        let data = ru_transaction.inner.rlp().to_vec();
        if self.bytes_sz + CHAIN_ID_LEN + data.len() > self.max_bytes_size {
            return Err(TransactionBufferError::BufferFull);
        }
        self.actions.push(Action::Sequence(SequenceAction {
            chain_id: ru_transaction.chain_id,
            data,
        }));
        self.bytes_sz += CHAIN_ID_LEN + data.len();
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    fn flush(&mut self) -> Vec<Action> {
        let ret = self.actions;
        *self = Self::new(self.max_bytes_size);
        ret
    }
}

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
    // Buffer for building up a set of `RollupTransaction`s to turn into an `Vec<Action>`
    buffer: TransactionBuffer,
    // The channel on which the bundler receives new `RollupTransaction`s from the `Collector`.
    rollup_transactions_rx: mpsc::Receiver<RollupTransaction>,
    // The channel on which the bundler sends new `Vec<Action>`s to the `Executor`.
    bundles_tx: mpsc::Sender<Vec<Action>>,
    // Timer for flushing the buffer at least once every block.
    // TODO: add block timer
    block_time: u64,
    // Duration to wait for backpressure before dropping the transaction as stale
    backpressure_timeout: u64,
}

impl Bundler {
    /// Initializes a new bundler instance
    pub fn new(
        rollup_transactions_rx: mpsc::Receiver<RollupTransaction>,
        sequencer_transactions_tx: mpsc::Sender<Vec<Action>>,
        max_bytes_size: usize,
        block_time: u64,
        backpressure_timeout: u64,
    ) -> Self {
        // TODO: add block timer
        let (status, _) = watch::channel(Status::new());
        Self {
            status,
            buffer: TransactionBuffer::new(max_bytes_size),
            rollup_transactions_rx,
            bundles_tx: sequencer_transactions_tx,
            block_time,
            backpressure_timeout,
        }
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    async fn flush_to_executor(&mut self) -> eyre::Result<()> {
        let actions = self.buffer.flush();
        // TODO: this is where backpressure resulting from the executor's transaction
        // submission would happen. need to properly report this
        // here
        if let Err(e) = self
            .bundles_tx
            .send_timeout(actions, Duration::from_millis(self.backpressure_timeout))
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
        let mut block_timer = time::interval(Duration::from_millis(self.block_time));
        loop {
            select! {
                Some(ru_transaction) = self.rollup_transactions_rx.recv() => {
                    // TODO: how to link from the rollup transaction's span to this buffer's span?
                    match self.buffer.push(ru_transaction) {
                        Ok(()) => {
                            debug!("rollup transaction added to bundler's buffer");
                            // TODO: log identifying info for this rollup transaction
                        }
                        Err(TransactionBufferError::RollupTransacionTooLarge) => {
                            warn!("rollup transaction is too large");
                            // TODO: log identifying info for this rollup transaction
                            // TODO: drop the rollup transaction
                        }
                        Err(TransactionBufferError::BufferFull) => {
                            debug!(
                                "bundler's buffer is full, flushing all buffered actions to the executor"
                            );
                            self.flush_to_executor().await?;
                        }
                    }
                }
                // TODO: add block timer
                _ = block_timer.tick() => {
                    debug!("bundler's block timer tick, flushing actions to the executor to ensure timely arrival of rollup transactions");
                    self.flush_to_executor().await?;
                }
            }
        }
    }
}
