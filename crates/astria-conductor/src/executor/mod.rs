use std::collections::HashMap;

use astria_sequencer_types::{
    ChainId,
    SequencerBlockData,
};
use color_eyre::eyre::{
    self,
    Result,
    WrapErr as _,
};
use hook::PreExecutionHook;
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

pub(crate) mod hook;
pub(crate) mod optimism;

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

    /// optional hook which is called to modify the rollup transaction list
    /// right before it's sent to the execution layer via `ExecuteBlock`.
    pre_execution_hook: Option<Box<dyn PreExecutionHook>>,
}

impl Executor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        server_addr: &str,
        chain_id: ChainId,
        cmd_rx: Receiver,
        shutdown: oneshot::Receiver<()>,
        hook: Option<Box<dyn PreExecutionHook>>,
    ) -> Result<Self> {
        let mut execution_rpc_client = ExecutionServiceClient::connect(server_addr.to_owned())
            .await
            .wrap_err("failed to create execution rpc client")?;
        let commitment_state = execution_rpc_client
            .call_get_commitment_state()
            .await
            .wrap_err("executor failed to get commitment state")?;

        info!(
            soft_block_hash = hex::encode(&commitment_state.soft.hash),
            firm_block_hash = hex::encode(&commitment_state.firm.hash),
            "initial execution commitment state",
        );

        Ok(Self {
            cmd_rx,
            shutdown,
            execution_rpc_client,
            chain_id,
            commitment_state,
            sequencer_hash_to_execution_block: HashMap::new(),
            pre_execution_hook: hook,
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
                    if let Err(e) = self.handle_executor_command(cmd).await {
                        error!(error.message = %e, "failed to handle executor command, breaking from executor loop");
                        break;
                    }
                }
            )
        }
        Ok(())
    }

    /// Handle a command received on the command channel.
    ///
    /// If this function returns an error, the executor will shut down.
    ///
    /// # Errors
    ///
    /// - if the command channel is closed unexpectedly
    /// - if execution or finalization of a block from celestia fails
    async fn handle_executor_command(&mut self, cmd: Option<ExecutorCommand>) -> eyre::Result<()> {
        let Some(cmd) = cmd else {
            return Err(eyre::eyre!(
                "cmd channel closed unexpectedly; shutting down"
            ));
        };

        match cmd {
            ExecutorCommand::FromSequencer {
                block,
            } => {
                let height = block.header().height.value();
                let block_subset =
                    SequencerBlockSubset::from_sequencer_block_data(*block, &self.chain_id);

                match self.execute_block(block_subset).await {
                    Ok(executed_block) => {
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
                }
            }

            ExecutorCommand::FromCelestia(blocks) => {
                self.execute_and_finalize_blocks_from_celestia(blocks)
                    .await
                    .wrap_err("failed to finalize block; stopping executor")?;
            }
        }

        Ok(())
    }

    /// Execute the sequencer block on the execution layer, returning the
    /// resulting execution block. If the block has already been executed, it
    /// returns the previously-computed execution block hash.
    #[instrument(skip(self), fields(sequencer_block_hash = ?block.block_hash, sequencer_block_height = block.header.height.value()))]
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Block> {
        if let Some(execution_block) = self
            .sequencer_hash_to_execution_block
            .get(&block.block_hash)
        {
            debug!(
                sequencer_block_height = block.header.height.value(),
                execution_hash = hex::encode(&execution_block.hash),
                "block already executed"
            );
            return Ok(execution_block.clone());
        }

        let prev_block_hash = self.commitment_state.soft.hash.clone();
        info!(
            sequencer_block_height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let rollup_transactions = if let Some(ref mut hook) = self.pre_execution_hook {
            hook.populate_rollup_transactions(block.rollup_transactions)
                .await?
        } else {
            block.rollup_transactions
        };

        let tx_count = rollup_transactions.len();
        let executed_block = self
            .execution_rpc_client
            .call_execute_block(prev_block_hash, rollup_transactions, timestamp)
            .await?;

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            execution_block_hash = hex::encode(&executed_block.hash),
            tx_count, "executed sequencer block",
        );

        // store block returned by execution client, as we need it to finalize the block later
        self.sequencer_hash_to_execution_block
            .insert(block.block_hash, executed_block.clone());

        Ok(executed_block)
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
        if blocks.is_empty() {
            info!("received a message from data availability without blocks; skipping execution");
            return Ok(());
        }
        for block in blocks.into_iter() {
            let sequencer_block_hash = block.block_hash;
            let maybe_executed_block = self
                .sequencer_hash_to_execution_block
                .get(&sequencer_block_hash)
                .cloned();
            match maybe_executed_block {
                Some(executed_block) => {
                    // this case means block has already been executed.
                    self.update_firm_commitment(executed_block)
                        .await
                        .wrap_err("executor failed to update firm commitment")?;
                    // remove the sequencer block hash from the map, as it's been firmly committed
                    self.sequencer_hash_to_execution_block
                        .remove(&sequencer_block_hash);
                }
                None => {
                    // this means either we didn't receive the block from the sequencer stream

                    // try executing the block as it hasn't been executed before
                    // execute_block will check if our namespace has txs; if so, it'll return the
                    // resulting execution block hash, otherwise None
                    let executed_block = self
                        .execute_block(block)
                        .await
                        .wrap_err("failed to execute block")?;

                    // when we execute a block received from da, nothing else has been executed on
                    // top of it, so we set FIRM and SOFT to this executed block
                    self.update_commitments(executed_block)
                        .await
                        .wrap_err("executor failed to update both commitments")?;
                    // remove the sequencer block hash from the map, as it's been firmly committed
                    self.sequencer_hash_to_execution_block
                        .remove(&sequencer_block_hash);
                }
            };
        }
        Ok(())
    }
}
