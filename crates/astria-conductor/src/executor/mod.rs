use std::collections::HashMap;

use color_eyre::eyre::{
    self,
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use proto::{
    generated::execution::v1alpha2::{
        execution_service_client::ExecutionServiceClient,
        Block,
        CommitmentState,
    },
    native::sequencer::v1alpha1::RollupId,
};
use sequencer_client::SequencerBlock;
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
};

use crate::data_availability::SequencerBlockSubset;

pub(crate) mod optimism;

mod client;
#[cfg(test)]
mod tests;

use client::ExecutionClientExt as _;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
pub(crate) type Receiver = UnboundedReceiver<ExecutorCommand>;

/// `ExecutorCommitmentState` tracks the firm and soft [`Block`]s from the
/// execution client. This is a utility type to avoid dealing with
/// Option<Block>s all over the place.
#[derive(Clone, Debug)]
pub(crate) struct ExecutorCommitmentState {
    firm: Block,
    soft: Block,
}

impl ExecutorCommitmentState {
    /// Creates a new `ExecutorCommitmentState` from a `CommitmentState`.
    /// `firm` and `soft` should never be `None`
    pub(crate) fn from_execution_client_commitment_state(data: CommitmentState) -> Self {
        let firm = data.firm.expect(
            "could not convert from CommitmentState to ExecutorCommitmentState. `firm` is None. \
             This should never happen.",
        );
        let soft = data.soft.expect(
            "could not convert from CommitmentState to ExecutorCommitmentState. `soft` is None. \
             This should never happen.",
        );

        Self {
            firm,
            soft,
        }
    }

    // pub(crate) fn soft_height(&self) -> u32 {
    //     self.soft.number
    // }

    pub(crate) fn firm_height(&self) -> u32 {
        self.firm.number
    }
}

// Given `Time`, convert to protobuf timestamp
fn convert_tendermint_to_prost_timestamp(value: Time) -> ProstTimestamp {
    use tendermint_proto::google::protobuf::Timestamp as TendermintTimestamp;
    let TendermintTimestamp {
        seconds,
        nanos,
    } = value.into();
    ProstTimestamp {
        seconds,
        nanos,
    }
}

#[derive(Debug)]
pub(crate) enum ExecutorCommand {
    /// used when a block is received from the subscription stream to sequencer
    FromSequencer { block: Box<SequencerBlock> },
    /// used when a block is received from the reader (Celestia)
    FromCelestia(Vec<SequencerBlockSubset>),
}

impl From<SequencerBlock> for ExecutorCommand {
    fn from(block: SequencerBlock) -> Self {
        Self::FromSequencer {
            block: Box::new(block),
        }
    }
}

pub(crate) struct NoRollupAddress;
pub(crate) struct WithRollupAddress(String);
pub(crate) struct NoRollupId;
pub(crate) struct WithRollupId(RollupId);
pub(crate) struct NoBlockChannel;
pub(crate) struct WithBlockChannel(Receiver);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

pub(crate) struct ExecutorBuilder<
    TBlockChannel = NoBlockChannel,
    TRollupAddress = NoRollupAddress,
    TRollupId = NoRollupId,
    TShutdown = NoShutdown,
> {
    block_channel: TBlockChannel,
    optimism_hook: Option<optimism::Handler>,
    rollup_address: TRollupAddress,
    rollup_id: TRollupId,
    sequencer_height_with_first_rollup_block: u32,
    shutdown: TShutdown,
}

impl ExecutorBuilder {
    fn new() -> Self {
        Self {
            block_channel: NoBlockChannel,
            optimism_hook: None,
            rollup_address: NoRollupAddress,
            rollup_id: NoRollupId,
            sequencer_height_with_first_rollup_block: 0,
            shutdown: NoShutdown,
        }
    }
}

impl ExecutorBuilder<WithBlockChannel, WithRollupAddress, WithRollupId, WithShutdown> {
    pub(crate) async fn build(self) -> eyre::Result<Executor> {
        let Self {
            rollup_id,
            block_channel,
            optimism_hook: pre_execution_hook,
            rollup_address,
            sequencer_height_with_first_rollup_block,
            shutdown,
        } = self;
        let WithRollupAddress(rollup_address) = rollup_address;
        let WithRollupId(rollup_id) = rollup_id;
        let WithBlockChannel(block_channel) = block_channel;
        let WithShutdown(shutdown) = shutdown;

        let mut execution_rpc_client = ExecutionServiceClient::connect(rollup_address)
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

        // let da_height_with_first_rollup_block = commitment_state.firm.number;

        Ok(Executor {
            block_channel,
            shutdown,
            execution_rpc_client,
            rollup_id,
            commitment_state,
            sequencer_height_with_first_rollup_block,
            // da_height_with_first_rollup_block,
            sequencer_hash_to_execution_block: HashMap::new(),
            pre_execution_hook,
        })
    }
}

impl<TBlockChannel, TRollupAddress, TRollupId, TShutdown>
    ExecutorBuilder<TBlockChannel, TRollupAddress, TRollupId, TShutdown>
{
    pub(crate) fn rollup_id(
        self,
        rollup_id: RollupId,
    ) -> ExecutorBuilder<TBlockChannel, TRollupAddress, WithRollupId, TShutdown> {
        let Self {
            block_channel,
            optimism_hook,
            rollup_address,
            sequencer_height_with_first_rollup_block,
            shutdown,
            ..
        } = self;
        ExecutorBuilder {
            block_channel,
            optimism_hook,
            rollup_address,
            rollup_id: WithRollupId(rollup_id),
            sequencer_height_with_first_rollup_block,
            shutdown,
        }
    }

    pub(crate) fn block_channel(
        self,
        block_channel: Receiver,
    ) -> ExecutorBuilder<WithBlockChannel, TRollupAddress, TRollupId, TShutdown> {
        let Self {
            optimism_hook,
            rollup_address,
            rollup_id,
            sequencer_height_with_first_rollup_block,
            shutdown,
            ..
        } = self;
        ExecutorBuilder {
            block_channel: WithBlockChannel(block_channel),
            optimism_hook,
            rollup_address,
            rollup_id,
            sequencer_height_with_first_rollup_block,
            shutdown,
        }
    }

    pub(crate) fn sequencer_height_with_first_rollup_block(
        mut self,
        sequencer_height_with_first_rollup_block: u32,
    ) -> Self {
        self.sequencer_height_with_first_rollup_block = sequencer_height_with_first_rollup_block;
        self
    }

    pub(crate) fn set_optimism_hook(mut self, handler: Option<optimism::Handler>) -> Self {
        self.optimism_hook = handler;
        self
    }

    pub(crate) fn rollup_address(
        self,
        rollup_address: &str,
    ) -> ExecutorBuilder<TBlockChannel, WithRollupAddress, TRollupId, TShutdown> {
        let Self {
            rollup_id,
            block_channel,
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            shutdown,
            ..
        } = self;
        ExecutorBuilder {
            block_channel,
            optimism_hook,
            rollup_address: WithRollupAddress(rollup_address.to_string()),
            rollup_id,
            sequencer_height_with_first_rollup_block,
            shutdown,
        }
    }

    pub(crate) fn shutdown(
        self,
        shutdown: oneshot::Receiver<()>,
    ) -> ExecutorBuilder<TBlockChannel, TRollupAddress, TRollupId, WithShutdown> {
        let Self {
            block_channel,
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            rollup_address,
            rollup_id,
            ..
        } = self;
        ExecutorBuilder {
            block_channel,
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            rollup_address,
            rollup_id,
            shutdown: WithShutdown(shutdown),
        }
    }
}

pub(crate) struct Executor {
    /// Channel on which executor commands are received.
    block_channel: Receiver,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionServiceClient<Channel>,

    /// Chain ID
    rollup_id: RollupId,

    /// Tracks SOFT and FIRM on the execution chain
    commitment_state: ExecutorCommitmentState,

    /// The first block height from sequencer used for a rollup block,
    /// executable block height & finalizable block height can be calcuated from
    /// this plus the commitment_state
    sequencer_height_with_first_rollup_block: u32,

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
    pre_execution_hook: Option<optimism::Handler>,
}

impl Executor {
    pub(super) fn builder() -> ExecutorBuilder {
        ExecutorBuilder::new()
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info!("starting executor run loop");
        loop {
            select!(
                biased;

                shutdown = &mut self.shutdown => {
                    let ret = if let Err(e) = shutdown {
                        let message = "shutdown channel closed unexpectedly";
                        error!(error = &e as &dyn std::error::Error, "{message}, shutting down");
                        Err(e).wrap_err(message)
                    } else {
                        info!("received_shutdown_signal, shutting down");
                        Ok(())
                    };
                    break ret;
                }

                cmd = self.block_channel.recv() => {
                    if let Err(e) = self.handle_executor_command(cmd).await {
                        let message = "failed handling executor command";
                        let error: &dyn std::error::Error = e.as_ref();
                        error!(error, "{message}, shutting down");
                        break Err(e).wrap_err(message);
                    }
                }
            );
        }
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
            bail!("cmd channel closed unexpectedly");
        };

        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        match cmd {
            ExecutorCommand::FromSequencer {
                block,
            } => {
                let block_subset =
                    SequencerBlockSubset::from_sequencer_block(*block, self.rollup_id);

                let executed_block = self
                    .execute_block(block_subset)
                    .await
                    .wrap_err("failed to execute block")?;
                self.update_soft_commitment(executed_block)
                    .await
                    .wrap_err("failed to update soft commitment")?;
            }

            ExecutorCommand::FromCelestia(blocks) => self
                .execute_and_finalize_blocks_from_celestia(blocks)
                .await
                .wrap_err("failed to finalize block")?,
        }
        Ok(())
    }

    /// Execute the sequencer block on the execution layer, returning the
    /// resulting execution block. If the block has already been executed, it
    /// returns the previously-computed execution block hash.
    #[instrument(skip(self), fields(sequencer_block_hash = ?block.block_hash, sequencer_block_height = block.header.height.value()))]
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Block> {
        let executable_block_height = self
            .calculate_executable_block_height()
            .wrap_err("failed calculating the next executable block height")?;
        let actual_block_height = block.header.height;
        ensure!(
            executable_block_height == actual_block_height,
            "received out-of-order block; expected `{executable_block_height}`, got \
             `{actual_block_height}`",
        );

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

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time);

        let rollup_transactions = if let Some(hook) = self.pre_execution_hook.as_mut() {
            hook.populate_rollup_transactions(block.transactions)
                .await
                .wrap_err("failed to populate rollup transactions before execution")?
        } else {
            block.transactions
        };

        let tx_count = rollup_transactions.len();
        let executed_block = self
            .execution_rpc_client
            .call_execute_block(prev_block_hash, rollup_transactions, timestamp)
            .await
            .wrap_err("failed to call execute_block")?;

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
        for block in blocks {
            let finalizable_block_height = self
                .calculate_finalizable_block_height()
                .wrap_err("failed calculating next finalizable block height")?;
            if block.header.height < finalizable_block_height {
                info!(
                    sequencer_block_height = %block.header.height,
                    finalized_block_height = %finalizable_block_height,
                    "received block which is already finalized; skipping finalization"
                );
                continue;
            }

            let sequencer_block_hash = block.block_hash;
            let maybe_executed_block = self
                .sequencer_hash_to_execution_block
                .get(&sequencer_block_hash)
                .cloned();

            if let Some(executed_block) = maybe_executed_block {
                // this case means block has already been executed.
                self.update_firm_commitment(executed_block)
                    .await
                    .wrap_err("executor failed to update firm commitment")?;
                // remove the sequencer block hash from the map, as it's been firmly committed
                self.sequencer_hash_to_execution_block
                    .remove(&sequencer_block_hash);
            } else {
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
            };
        }
        Ok(())
    }

    // Returns the next sequencer block height which can be executed on the rollup
    pub(crate) fn calculate_executable_block_height(&self) -> Result<tendermint::block::Height> {
        let Some(executable_block_height) = calculate_sequencer_block_height(
            self.sequencer_height_with_first_rollup_block,
            self.commitment_state.soft.number,
        ) else {
            bail!(
                "encountered overflow when calculating executable block height; sequencer height \
                 with first rollup block: {}, height recorded in soft commitment state: {}",
                self.sequencer_height_with_first_rollup_block,
                self.commitment_state.soft.number,
            );
        };

        Ok(executable_block_height.into())
    }

    // Returns the lowest sequencer block height which can finalized on the rollup.
    pub(crate) fn calculate_finalizable_block_height(&self) -> Result<tendermint::block::Height> {
        let Some(finalizable_block_height) = calculate_sequencer_block_height(
            self.sequencer_height_with_first_rollup_block,
            self.commitment_state.firm.number,
        ) else {
            bail!(
                "encountered overflow when calculating finalizable block height; sequencer height \
                 with first rollup block: {}, height recorded in firm commitment state: {}",
                self.sequencer_height_with_first_rollup_block,
                self.commitment_state.firm.number,
            );
        };

        Ok(finalizable_block_height.into())
    }
}

/// Calculates the sequencer block height for a given rollup height.
///
/// This function assumes that sequencer heights and rollup heights increment in
/// lockstep. `initial_sequencer_height` contains the first rollup height, while
/// `rollup_height` is the height of a rollup block. That makes
/// `initial_sequencer_height + rollup_height` the corresponding sequencer
/// height.
fn calculate_sequencer_block_height(
    initial_sequencer_height: u32,
    rollup_height: u32,
) -> Option<u32> {
    initial_sequencer_height.checked_add(rollup_height)
}
