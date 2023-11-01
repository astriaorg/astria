use std::collections::HashMap;

use astria_sequencer_types::{
    ChainId,
    SequencerBlockData,
};
use color_eyre::eyre::{
    self,
    eyre,
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use proto::generated::execution::v1alpha2::{
    execution_service_client::ExecutionServiceClient,
    Block,
};
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
use tonic::transport::Channel;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    execution_client::ExecutionClientExt,
    types::{
        ExecutorCommitmentState,
        SequencerBlockSubset,
    },
};

#[cfg(test)]
mod tests;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
pub(crate) type Receiver = UnboundedReceiver<ExecutorCommand>;

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

impl From<SequencerBlockData> for ExecutorCommand {
    fn from(block: SequencerBlockData) -> Self {
        Self::FromSequencer {
            block: Box::new(block),
        }
    }
}

pub(crate) struct Executor {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionServiceClient<Channel>,

    /// Chain ID
    chain_id: ChainId,

    /// Tracks SOFT and FIRM on the execution chain
    commitment_state: ExecutorCommitmentState,

    /// Tracks the height of the next sequencer block that can be executed
    executable_block_height: u32,

    /// map of sequencer block hash to execution block
    ///
    /// this is required because when we receive sequencer blocks (from network or DA),
    /// we only know the sequencer block hash, but not the execution block hash,
    /// as the execution block hash is created by executing the block.
    /// as well, the execution layer is not aware of the sequencer block hash.
    /// we need to track the mapping of sequencer block hash -> execution block
    /// so that we can mark the block as final on the execution layer when
    /// we receive a finalized sequencer block.
    sequencer_hash_to_execution_block: HashMap<Hash, Block>,

    /// Chose to execute empty blocks or not
    disable_empty_block_execution: bool,
}

impl Executor {
    pub(crate) async fn new(
        server_addr: &str,
        chain_id: ChainId,
        disable_empty_block_execution: bool,
        init_sequencer_height: u32,
        cmd_rx: Receiver,
        shutdown: oneshot::Receiver<()>,
    ) -> Result<Self> {
        let mut execution_rpc_client = ExecutionServiceClient::connect(server_addr.to_owned())
            .await
            .wrap_err("failed to create execution rpc client")?;
        let commitment_state = execution_rpc_client
            .call_get_commitment_state()
            .await
            .wrap_err("executor failed to get commitment state")?;

        // The `executable_block_height` is the height of the fist sequencer block
        // that can be executed on top of the rollup state. The `init_sequencer_height`
        // is set when the rollup is first created and let's us know that this
        // rollup's genesis block is in block K of the sequencer chain.
        // The `commitment_state.soft.number` is the block height of the most
        // recently executed block on the rollup and is pulled from the rollup
        // the Conductor is associated with on startup of the Conductor (N
        // blocks have already been executed on the rollup).
        // `executable_block_height` represents where the Condutor sync should
        // start:
        // `executable_block_height` = sequencer block K + executed block N
        // By setting this value we prevent the reexecution of blocks on the
        // rollup. If block M is the most recent sequencer block, we then know
        // that we need to sync blocks from `executable_block_height` to M.
        let executable_block_height = commitment_state.soft.number + init_sequencer_height;

        Ok(Self {
            cmd_rx,
            shutdown,
            execution_rpc_client,
            chain_id,
            commitment_state,
            executable_block_height,
            sequencer_hash_to_execution_block: HashMap::new(),
            disable_empty_block_execution,
        })
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
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

                            let executed_block_result = self.execute_block(block_subset).await;
                            match executed_block_result {
                                Ok(Some(executed_block)) => {
                                    if let Err(e) = self.update_soft_commitment(executed_block.clone()).await {
                                        error!(
                                            height = height,
                                            error = ?e,
                                            "failed to update soft commitment"
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        height = height,
                                        error = ?e,
                                        "failed to execute block"
                                    );
                                }
                                // execution was skipped
                                Ok(None) => {}
                            }
                        }

                        ExecutorCommand::FromCelestia(blocks) => {
                            if let Err(e) = self
                                .execute_and_finalize_blocks_from_celestia(blocks)
                                .await
                            {
                                let error: &(dyn std::error::Error + 'static) = e.as_ref();
                                error!(error, "failed to finalize block; stopping executor");
                                break;
                            }
                        }
                    }
                }
            )
        }
        Ok(())
    }

    /// Checks for relevant transactions in the SequencerBlock and attempts
    /// to execute them via the execution service function ExecuteBlock.
    /// If there are relevant transactions that successfully execute,
    /// it returns the resulting execution block hash.
    /// If the block has already been executed, it returns the previously-computed
    /// execution block hash.
    /// if there are no relevant transactions in the SequencerBlock, it returns None.
    /// If the block is out of order, it returns an error.
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Option<Block>> {
        if self.disable_empty_block_execution && block.rollup_transactions.is_empty() {
            debug!(
                sequencer_block_height = block.header.height.value(),
                "no transactions in block, skipping execution"
            );
            return Ok(None);
        }

        if self.executable_block_height as u64 != block.header.height.value() {
            error!(
                sequencer_block_height = block.header.height.value(),
                executable_block_height = self.executable_block_height,
                "block received out of order;"
            );
            return Err(eyre!("block received out of order"));
        }

        if let Some(execution_block) = self
            .sequencer_hash_to_execution_block
            .get(&block.block_hash)
        {
            debug!(
                sequencer_block_height = block.header.height.value(),
                execution_hash = hex::encode(&execution_block.hash),
                "block already executed"
            );
            return Ok(Some(execution_block.clone()));
        }

        let prev_block_hash = self.commitment_state.soft.hash.clone();
        info!(
            sequencer_block_height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let executed_block = self
            .execution_rpc_client
            .call_execute_block(prev_block_hash, block.rollup_transactions, timestamp)
            .await?;
        self.executable_block_height += 1;

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&executed_block.hash),
            "executed sequencer block",
        );

        // store block returned by execution client, as we need it to finalize the block later
        self.sequencer_hash_to_execution_block
            .insert(block.block_hash, executed_block.clone());

        Ok(Some(executed_block))
    }

    /// Updates the commitment state on the execution layer.
    /// Updates the local `commitment_state` with the new values.
    async fn update_commitment_states(&mut self, firm: Block, soft: Block) -> Result<()> {
        let new_commitment_state = self
            .execution_rpc_client
            .call_update_commitment_state(firm, soft)
            .await
            .wrap_err("executor failed to update commitment state")?;
        self.commitment_state = new_commitment_state;
        Ok(())
    }

    /// Updates both firm and soft commitments.
    async fn update_commitments(&mut self, block: Block) -> Result<()> {
        self.update_commitment_states(block.clone(), block)
            .await
            .wrap_err("executor failed to update both commitments")?;
        Ok(())
    }

    /// Updates only firm commitment and leaves soft commitment the same.
    async fn update_firm_commitment(&mut self, firm: Block) -> Result<()> {
        self.update_commitment_states(firm, self.commitment_state.soft.clone())
            .await
            .wrap_err("executor failed to update firm commitment")?;
        Ok(())
    }

    /// Updates only soft commitment and leaves firm commitment the same.
    async fn update_soft_commitment(&mut self, soft: Block) -> Result<()> {
        self.update_commitment_states(self.commitment_state.firm.clone(), soft)
            .await
            .wrap_err("executor failed to update soft commitment")?;
        Ok(())
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
        let maybe_executed_block = self
            .sequencer_hash_to_execution_block
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_executed_block {
            Some(executed_block) => {
                // this case means block has already been executed.
                self.update_firm_commitment(executed_block.clone())
                    .await
                    .wrap_err("executor failed to update firm commitment")?;
                // remove the sequencer block hash from the map, as it's been firmly committed
                self.sequencer_hash_to_execution_block
                    .remove(&block.block_hash);
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
                let Some(executed_block) = self
                    .execute_block(block.clone())
                    .await
                    .wrap_err("failed to execute block")?
                else {
                    // no txs for our namespace, nothing to do
                    debug!("execute_block returned None; skipping call_update_commitment_state");
                    return Ok(());
                };

                // when we execute a block received from da, nothing else has been executed on top
                // of it, so we set FIRM and SOFT to this executed block
                self.update_commitments(executed_block)
                    .await
                    .wrap_err("executor failed to update both commitments")?;
                // remove the sequencer block hash from the map, as it's been firmly committed
                self.sequencer_hash_to_execution_block
                    .remove(&block.block_hash);
            }
        };
        Ok(())
    }

    pub(crate) fn get_executable_block_height(&self) -> u32 {
        self.executable_block_height
    }
}
