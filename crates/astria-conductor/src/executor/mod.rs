use std::collections::{
    HashMap,
    VecDeque,
};

use astria_sequencer_types::{
    ChainId,
    SequencerBlockData,
};
use color_eyre::eyre::{
    self,
    Result,
    WrapErr as _,
};
use ethers::prelude::*;
use optimism::{
    contract::{
        OptimismPortal,
        TransactionDepositedFilter,
    },
    watcher::convert_deposit_event_to_deposit_tx,
    OptimismDepositedTransactionRequest,
};
use prost_types::Timestamp as ProstTimestamp;
use tendermint::{
    Hash,
    Time,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
        oneshot,
    },
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    execution_client::{
        ExecutionClient,
        ExecutionRpcClient,
    },
    types::SequencerBlockSubset,
};

#[cfg(test)]
mod tests;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

// Given `Time`, convert to protobuf timestamp
fn convert_tendermint_to_prost_timestamp(value: Time) -> Result<ProstTimestamp> {
    use tendermint_proto::google::protobuf::Timestamp as TendermintTimestamp;
    let TendermintTimestamp {
        seconds,
        nanos,
    } = value.into();
    Ok(ProstTimestamp {
        seconds,
        nanos,
    })
}

#[derive(Debug)]
pub(crate) enum ExecutorCommand {
    /// used when a block is received from the subscription stream to sequencer
    FromSequencer { block: Box<SequencerBlockData> },
    /// used when a block is received from the reader (Celestia)
    FromCelestia(Vec<SequencerBlockSubset>),
}

pub(crate) struct Executor {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionRpcClient,

    /// Chain ID
    chain_id: ChainId,

    /// Tracks the state of the execution chain
    execution_state: Vec<u8>,

    /// map of sequencer block hash to execution block hash
    ///
    /// this is required because when we receive sequencer blocks (from network or DA),
    /// we only know the sequencer block hash, but not the execution block hash,
    /// as the execution block hash is created by executing the block.
    /// as well, the execution layer is not aware of the sequencer block hash.
    /// we need to track the mapping of sequencer block hash -> execution block hash
    /// so that we can mark the block as final on the execution layer when
    /// we receive a finalized sequencer block.
    sequencer_hash_to_execution_hash: HashMap<Hash, Vec<u8>>,

    /// Chose to execute empty blocks or not
    disable_empty_block_execution: bool,

    optimism_portal_contract: OptimismPortal<Provider<Ws>>,
    queued_deposit_txs: VecDeque<OptimismDepositedTransactionRequest>,
}

impl Executor {
    pub(crate) async fn new(
        server_addr: &str,
        chain_id: ChainId,
        disable_empty_block_execution: bool,
        cmd_rx: Receiver,
        shutdown: oneshot::Receiver<()>,
        optimism_portal_contract: OptimismPortal<Provider<Ws>>,
    ) -> Result<Self> {
        let mut execution_rpc_client = ExecutionRpcClient::new(server_addr)
            .await
            .wrap_err("failed to create execution rpc client")?;
        let init_state_response = execution_rpc_client
            .call_init_state()
            .await
            .wrap_err("could not initialize execution rpc client state")?;
        let execution_state = init_state_response.block_hash;
        Ok(Self {
            cmd_rx,
            shutdown,
            execution_rpc_client,
            chain_id,
            execution_state,
            sequencer_hash_to_execution_hash: HashMap::new(),
            disable_empty_block_execution,
            optimism_portal_contract,
            queued_deposit_txs: VecDeque::new(),
        })
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        // TODO: configure starting block
        let events = self
            .optimism_portal_contract
            .event::<TransactionDepositedFilter>()
            .from_block(1);
        let mut stream = events.stream().await?.with_meta();

        loop {
            select!(
                biased;

                shutdown = &mut self.shutdown => {
                    match shutdown {
                        Err(e) => warn!(error.message = %e, "shutdown channel return with error; shutting down"),
                        Ok(()) => info!("received shutdown signal; shutting down"),
                    }
                    break;
                }

                res = stream.next() => {
                    match res {
                        None => {
                            warn!("event stream closed unexpectedly; shutting down");
                            break;
                        }
                        Some(Ok((event, meta))) => {
                            let deposit_tx = match convert_deposit_event_to_deposit_tx(event, meta.block_hash, meta.log_index) {
                                Ok(deposit_tx) => deposit_tx,
                                Err(e) => {
                                    warn!(error.message = %e, "failed to convert deposit event to deposit tx");
                                    continue;
                                }
                            };
                            info!(
                                ethereum_block_hash = ?meta.block_hash,
                                ethereum_log_index = ?meta.log_index,
                                "queuing deposit transaction",
                            );
                            self.queued_deposit_txs.push_back(deposit_tx);

                            // HACK just execute stuff for now to see if it works
                            let mut deposit_txs = Vec::new();
                            while let Some(deposit_tx) = self.queued_deposit_txs.pop_front() {
                                deposit_txs.push(deposit_tx);
                            }
                            let prefix: Vec<u8> = vec![0x7e];
                            let deposit_txs = deposit_txs
                                .into_iter()
                                .map(|tx| {
                                    [prefix.clone(), tx.rlp().to_vec()].concat()
                                })
                                .collect::<Vec<Vec<u8>>>();

                            let response = self
                                .execution_rpc_client
                                .call_do_block(self.execution_state.clone(), deposit_txs, Some(convert_tendermint_to_prost_timestamp(tendermint::Time::now())?))
                                .await?;
                            self.execution_state = response.block_hash.clone();

                        }
                        Some(Err(e)) => {
                            warn!(error.message = %e, "event stream returned with error; shutting down");
                            break;
                        }
                    }
                }

                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        error!("cmd channel closed unexpectedly; shutting down");
                        break;
                    };
                    match cmd {
                        ExecutorCommand::FromSequencer {
                            block,
                        } => {
                            let height = block.header().height.value();
                            let block_subset =
                                SequencerBlockSubset::from_sequencer_block_data(*block, &self.chain_id);

                            if let Err(e) = self.execute_block(block_subset).await {
                                error!(
                                    sequencer_block_height = height,
                                    error = ?e,
                                    "failed to execute block"
                                );
                            }
                        }

                        ExecutorCommand::FromCelestia(blocks) => {
                            if let Err(e) = self
                                .execute_and_finalize_blocks_from_celestia(blocks)
                                .await
                            {
                                error!(
                                    error.message = %e,
                                    error.cause = ?e,
                                    "failed to finalize block; stopping executor"
                                );
                                break;
                            }
                        }
                    }
                }
            )
        }
        Ok(())
    }

    /// checks for relevant transactions in the SequencerBlock and attempts
    /// to execute them via the execution service function DoBlock.
    /// if there are relevant transactions that successfully execute,
    /// it returns the resulting execution block hash.
    /// if the block has already been executed, it returns the previously-computed
    /// execution block hash.
    /// if there are no relevant transactions in the SequencerBlock, it returns None.
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Option<Vec<u8>>> {
        if self.disable_empty_block_execution && block.rollup_transactions.is_empty() {
            debug!(
                sequencer_block_height = block.header.height.value(),
                "no transactions in block, skipping execution"
            );
            return Ok(None);
        }

        if let Some(execution_hash) = self.sequencer_hash_to_execution_hash.get(&block.block_hash) {
            debug!(
                sequencer_block_height = block.header.height.value(),
                execution_hash = hex::encode(execution_hash),
                "block already executed"
            );
            return Ok(Some(execution_hash.clone()));
        }

        let prev_block_hash = self.execution_state.clone();
        info!(
            sequencer_block_height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        // gather and encode deposit transactions
        let mut deposit_txs = Vec::new();
        while let Some(deposit_tx) = self.queued_deposit_txs.pop_front() {
            deposit_txs.push(deposit_tx);
        }
        let deposit_txs = deposit_txs
            .into_iter()
            .map(|tx| tx.rlp().to_vec())
            .collect::<Vec<Vec<u8>>>();

        let rollup_transactions = [deposit_txs, block.rollup_transactions].concat();

        let response = self
            .execution_rpc_client
            .call_do_block(prev_block_hash, rollup_transactions, Some(timestamp))
            .await?;
        self.execution_state = response.block_hash.clone();

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&response.block_hash),
            "executed sequencer block",
        );
        self.sequencer_hash_to_execution_hash
            .insert(block.block_hash, response.block_hash.clone());

        Ok(Some(response.block_hash))
    }

    async fn execute_and_finalize_blocks_from_celestia(
        &mut self,
        blocks: Vec<SequencerBlockSubset>,
    ) -> Result<()> {
        // FIXME: actually process all blocks.
        let Some(block) = blocks.get(0).cloned() else {
            info!("received a message from data availability without blocks; skipping execution");
            return Ok(());
        };
        let sequencer_block_hash = block.block_hash;
        let maybe_execution_block_hash = self
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_execution_block_hash {
            Some(execution_block_hash) => {
                self.finalize_block(execution_block_hash, sequencer_block_hash)
                    .await?;
            }
            None => {
                // this means either:
                // - we didn't receive the block from the sequencer stream, or
                // - we received it, but the sequencer block didn't contain
                // any transactions for this rollup namespace, thus nothing was executed
                // on receiving this block.

                // try executing the block as it hasn't been executed before
                // execute_block will check if our namespace has txs; if so, it'll return the
                // resulting execution block hash, otherwise None
                let Some(execution_block_hash) = self
                    .execute_block(block)
                    .await
                    .wrap_err("failed to execute block")?
                else {
                    // no txs for our namespace, nothing to do
                    debug!("execute_block returned None; skipping finalize_block");
                    return Ok(());
                };

                // finalize the block after it's been executed
                self.finalize_block(execution_block_hash, sequencer_block_hash)
                    .await?;
            }
        };
        Ok(())
    }

    /// This function finalizes the given execution block on the execution layer by calling
    /// the execution service's FinalizeBlock function.
    /// note that this function clears the respective entry in the
    /// `sequencer_hash_to_execution_hash` map.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - the call to the execution service's FinalizeBlock function fails
    #[instrument(ret, err, skip_all, fields(
        execution_block_hash = hex::encode(&execution_block_hash),
        sequencer_block_hash = hex::encode(sequencer_block_hash),
    ))]
    async fn finalize_block(
        &mut self,
        execution_block_hash: Vec<u8>,
        sequencer_block_hash: Hash,
    ) -> Result<()> {
        self.execution_rpc_client
            .call_finalize_block(execution_block_hash)
            .await
            .wrap_err("failed to finalize block")?;
        self.sequencer_hash_to_execution_hash
            .remove(&sequencer_block_hash);
        Ok(())
    }
}
