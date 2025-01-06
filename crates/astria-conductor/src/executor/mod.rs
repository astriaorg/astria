use std::collections::HashMap;

use astria_core::{
    execution::v1::{
        Block,
        CommitmentState,
    },
    primitive::v1::RollupId,
    sequencerblock::v1::block::{
        FilteredSequencerBlock,
        FilteredSequencerBlockParts,
    },
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use bytes::Bytes;
use sequencer_client::tendermint::{
    block::Height as SequencerHeight,
    Time as TendermintTime,
};
use tokio::{
    select,
    sync::{
        mpsc,
        watch::error::RecvError,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    debug_span,
    error,
    info,
    instrument,
};

use crate::{
    celestia::ReconstructedBlock,
    conductor::ExitReason,
    config::CommitLevel,
    metrics::Metrics,
};

mod builder;
pub(crate) mod channel;

pub(crate) use builder::Builder;
use channel::soft_block_channel;

mod client;
mod state;
#[cfg(test)]
mod tests;

pub(super) use client::Client;
use state::StateReceiver;

use self::state::StateSender;

type CelestiaHeight = u64;

#[derive(Clone, Debug)]
pub(crate) struct StateNotInit;
#[derive(Clone, Debug)]
pub(crate) struct StateIsInit;

#[derive(Debug)]
pub(crate) enum StopHeightExceded {
    Celestia { firm_height: u64, stop_height: u64 },
    Sequencer { soft_height: u64, stop_height: u64 },
}

impl StopHeightExceded {
    fn celestia(firm_height: u64, stop_height: u64) -> Self {
        StopHeightExceded::Celestia {
            firm_height,
            stop_height,
        }
    }

    fn sequencer(soft_height: u64, stop_height: u64) -> Self {
        StopHeightExceded::Sequencer {
            soft_height,
            stop_height,
        }
    }
}

impl std::fmt::Display for StopHeightExceded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopHeightExceded::Celestia {
                firm_height,
                stop_height,
            } => write!(
                f,
                "firm block height {firm_height} exceded stop height {stop_height}",
            ),
            StopHeightExceded::Sequencer {
                soft_height,
                stop_height,
            } => write!(
                f,
                "soft block height {soft_height} exceded stop height {stop_height}",
            ),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FirmSendError {
    #[error("executor was configured without firm commitments")]
    NotSet,
    #[error("failed sending blocks to executor")]
    Channel {
        #[from]
        source: mpsc::error::SendError<ReconstructedBlock>,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FirmTrySendError {
    #[error("executor was configured without firm commitments")]
    NotSet,
    #[error("failed sending blocks to executor")]
    Channel {
        #[from]
        source: mpsc::error::TrySendError<ReconstructedBlock>,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SoftSendError {
    #[error("executor was configured without soft commitments")]
    NotSet,
    #[error("failed sending blocks to executor")]
    Channel { source: Box<channel::SendError> },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SoftTrySendError {
    #[error("executor was configured without firm commitments")]
    NotSet,
    #[error("failed sending blocks to executor")]
    Channel {
        source: Box<channel::TrySendError<FilteredSequencerBlock>>,
    },
}

/// A handle to the executor.
///
/// To be useful, [`Handle<StateNotInit>::wait_for_init`] must be called in
/// order to obtain a [`Handle<StateInit>`]. This is to ensure that the executor
/// state was primed before using its other methods. See [`State`] for more
/// information.
#[derive(Debug, Clone)]
pub(crate) struct Handle<TStateInit = StateNotInit> {
    firm_blocks: Option<mpsc::Sender<ReconstructedBlock>>,
    soft_blocks: Option<channel::Sender<FilteredSequencerBlock>>,
    state: StateReceiver,
    _state_init: TStateInit,
}

impl<T: Clone> Handle<T> {
    #[instrument(skip_all, err)]
    pub(crate) async fn wait_for_init(&mut self) -> eyre::Result<Handle<StateIsInit>> {
        self.state.wait_for_init().await.wrap_err(
            "executor state channel terminated while waiting for the state to initialize",
        )?;
        let Self {
            firm_blocks,
            soft_blocks,
            state,
            ..
        } = self.clone();
        Ok(Handle {
            firm_blocks,
            soft_blocks,
            state,
            _state_init: StateIsInit,
        })
    }
}

impl Handle<StateIsInit> {
    #[instrument(skip_all, err)]
    pub(crate) async fn send_firm_block(
        self,
        block: ReconstructedBlock,
    ) -> Result<(), FirmSendError> {
        let sender = self.firm_blocks.as_ref().ok_or(FirmSendError::NotSet)?;
        sender.send(block).await?;
        Ok(())
    }

    pub(crate) fn try_send_firm_block(
        &self,
        block: ReconstructedBlock,
    ) -> Result<(), FirmTrySendError> {
        let sender = self.firm_blocks.as_ref().ok_or(FirmTrySendError::NotSet)?;
        sender.try_send(block)?;
        Ok(())
    }

    #[instrument(skip_all, err)]
    pub(crate) async fn send_soft_block_owned(
        self,
        block: FilteredSequencerBlock,
    ) -> Result<(), SoftSendError> {
        let chan = self.soft_blocks.as_ref().ok_or(SoftSendError::NotSet)?;
        chan.send(block)
            .await
            .map_err(|source| SoftSendError::Channel {
                source: Box::new(source),
            })?;
        Ok(())
    }

    pub(crate) fn try_send_soft_block(
        &self,
        block: FilteredSequencerBlock,
    ) -> Result<(), SoftTrySendError> {
        let chan = self.soft_blocks.as_ref().ok_or(SoftTrySendError::NotSet)?;
        chan.try_send(block)
            .map_err(|source| SoftTrySendError::Channel {
                source: Box::new(source),
            })?;
        Ok(())
    }

    pub(crate) fn next_expected_firm_sequencer_height(&mut self) -> SequencerHeight {
        self.state.next_expected_firm_sequencer_height()
    }

    pub(crate) fn next_expected_soft_sequencer_height(&mut self) -> SequencerHeight {
        self.state.next_expected_soft_sequencer_height()
    }

    #[instrument(skip_all)]
    pub(crate) async fn next_expected_soft_height_if_changed(
        &mut self,
    ) -> Result<SequencerHeight, RecvError> {
        self.state.next_expected_soft_height_if_changed().await
    }

    pub(crate) fn rollup_id(&mut self) -> RollupId {
        self.state.rollup_id()
    }

    pub(crate) fn celestia_base_block_height(&mut self) -> CelestiaHeight {
        self.state.celestia_base_block_height()
    }

    pub(crate) fn celestia_block_variance(&mut self) -> u64 {
        self.state.celestia_block_variance()
    }

    pub(crate) fn sequencer_chain_id(&self) -> String {
        self.state.sequencer_chain_id()
    }

    pub(crate) fn celestia_chain_id(&self) -> String {
        self.state.celestia_chain_id()
    }
}

pub(crate) struct Executor {
    /// The execution client driving the rollup.
    client: Client,

    /// The mode under which this executor (and hence conductor) runs.
    mode: CommitLevel,

    /// The channel of which this executor receives blocks for executing
    /// firm commitments.
    /// Only set if `mode` is `FirmOnly` or `SoftAndFirm`.
    firm_blocks: Option<mpsc::Receiver<ReconstructedBlock>>,

    /// The channel of which this executor receives blocks for executing
    /// soft commitments.
    /// Only set if `mode` is `SoftOnly` or `SoftAndFirm`.
    soft_blocks: Option<channel::Receiver<FilteredSequencerBlock>>,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,

    /// Tracks the status of the execution chain.
    state: StateSender,

    /// Tracks rollup blocks by their rollup block numbers.
    ///
    /// Required to mark firm blocks received from celestia as executed
    /// without re-executing on top of the rollup node.
    blocks_pending_finalization: HashMap<u32, Block>,

    /// The maximum permitted spread between firm and soft blocks.
    max_spread: Option<usize>,

    metrics: &'static Metrics,
}

impl Executor {
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<ExitReason> {
        select!(
            () = self.shutdown.clone().cancelled_owned() => {
                return report_exit(Ok(
                    ExitReason::ShutdownSignal
                ), "aborting initialization due to shutdown signal");
            }
            res = self.init() => {
                res.wrap_err("initialization failed")?;
            }
        );

        let reason = loop {
            if self.firm_blocks.is_none() && self.soft_blocks.is_none() {
                break Ok(ExitReason::ChannelsClosed);
            }

            let spread_not_too_large = !self.is_spread_too_large();
            if spread_not_too_large {
                if let Some(channel) = self.soft_blocks.as_mut() {
                    channel.fill_permits();
                }
            }

            select!(
                biased;

                () = self.shutdown.cancelled() => {
                    break Ok(ExitReason::ShutdownSignal);
                }

                Some(block) = async { self.firm_blocks.as_mut().unwrap().recv().await },
                              if self.firm_blocks.is_some() =>
                {
                    debug_span!("conductor::Executor::run_until_stopped").in_scope(||debug!(
                        block.height = %block.sequencer_height(),
                        block.hash = %telemetry::display::base64(&block.block_hash),
                        "received block from celestia reader",
                    ));
                    match self.execute_firm(block).await {
                        Ok(None) => {},
                        Ok(Some(stop_height_exceded)) => {
                            break Ok(ExitReason::StopHeightExceded(stop_height_exceded));
                        }
                        Err(error) => {
                            break Err(error).wrap_err("failed executing firm block");
                        }
                    }
                }

                Some(block) = async { self.soft_blocks.as_mut().unwrap().recv().await },
                              if self.soft_blocks.is_some() && spread_not_too_large =>
                {
                    debug_span!("conductor::Executor::run_until_stopped").in_scope(||debug!(
                        block.height = %block.height(),
                        block.hash = %telemetry::display::base64(&block.block_hash()),
                        "received block from sequencer reader",
                    ));
                    match self.execute_soft(block).await {
                        Ok(None) => {},
                        Ok(Some(stop_height_exceded)) => {
                            break Ok(ExitReason::StopHeightExceded(stop_height_exceded));
                        }
                        Err(error) => {
                            break Err(error).wrap_err("failed executing soft block");
                        }
                    }
                }
            );
        };

        // XXX: explicitly setting the message (usually implicitly set by tracing)
        let message = "shutting down";
        report_exit(reason, message)
    }

    /// Runs the init logic that needs to happen before [`Executor`] can enter its main loop.
    #[instrument(skip_all, err)]
    async fn init(&mut self) -> eyre::Result<()> {
        self.set_initial_node_state()
            .await
            .wrap_err("failed setting initial rollup node state")?;

        let max_spread: usize = self.calculate_max_spread();
        self.max_spread.replace(max_spread);
        if let Some(channel) = self.soft_blocks.as_mut() {
            channel.set_capacity(max_spread);
            info!(
                max_spread,
                "setting capacity of soft blocks channel to maximum permitted firm<>soft \
                 commitment spread (this has no effect if conductor is set to perform soft-sync \
                 only)"
            );
        }

        Ok(())
    }

    /// Calculates the maximum allowed spread between firm and soft commitments heights.
    ///
    /// The maximum allowed spread is taken as `max_spread = variance * 6`, where `variance`
    /// is the `celestia_block_variance` as defined in the rollup node's genesis that this
    /// executor/conductor talks to.
    ///
    /// The heuristic 6 is the largest number of Sequencer heights that will be found at
    /// one Celestia height.
    ///
    /// # Panics
    /// Panics if the `u32` underlying the celestia block variance tracked in the state could
    /// not be converted to a `usize`. This should never happen on any reasonable architecture
    /// that Conductor will run on.
    fn calculate_max_spread(&self) -> usize {
        usize::try_from(self.state.celestia_block_variance())
            .expect("converting a u32 to usize should work on any architecture conductor runs on")
            .saturating_mul(6)
    }

    /// Returns if the spread between firm and soft commitment heights in the tracked state is too
    /// large.
    ///
    /// Always returns `false` if this executor was configured to run without firm commitments.
    ///
    /// # Panics
    ///
    /// Panics if called before [`Executor::init`] because `max_spread` must be set.
    fn is_spread_too_large(&self) -> bool {
        if self.firm_blocks.is_none() {
            return false;
        }
        let (next_firm, next_soft) = {
            let next_firm = self.state.next_expected_firm_sequencer_height().value();
            let next_soft = self.state.next_expected_soft_sequencer_height().value();
            (next_firm, next_soft)
        };

        let is_too_far_ahead = usize::try_from(next_soft.saturating_sub(next_firm))
            .map(|spread| {
                spread
                    >= self
                        .max_spread
                        .expect("executor must be initalized and this field set")
            })
            .unwrap_or(false);

        if is_too_far_ahead {
            debug!("soft blocks are too far ahead of firm; skipping soft blocks");
        }
        is_too_far_ahead
    }

    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::base64(&block.block_hash()),
        block.height = block.height().value(),
        err,
    ))]
    async fn execute_soft(
        &mut self,
        block: FilteredSequencerBlock,
    ) -> eyre::Result<Option<StopHeightExceded>> {
        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        let executable_block = ExecutableBlock::from_sequencer(block, self.state.rollup_id());

        // Stop executing soft blocks at the sequencer stop block height (exclusive). If we are also
        // executing firm blocks, we let execution continue since one more firm block will be
        // executed before `execute_firm` initiates a restart. If we are in soft-only mode, we
        // return a `StopHeightExceded::Sequencer` error to signal a restart.
        if self.is_soft_block_height_exceded(&executable_block) {
            if self.mode.is_with_firm() {
                // If we are continuing to execute firm blocks, we should close the soft block
                // channel so as to not keep hitting the sequencer RPC endpoint.
                if let Some(channel) = self.soft_blocks.take() {
                    drop(channel);
                }
                self.soft_blocks = None;
                return Ok(None);
            }
            return Ok(Some(StopHeightExceded::sequencer(
                executable_block.height.value(),
                self.state.sequencer_stop_block_height(),
            )));
        }

        let expected_height = self.state.next_expected_soft_sequencer_height();
        match executable_block.height.cmp(&expected_height) {
            std::cmp::Ordering::Less => {
                info!(
                    expected_height.sequencer_block = %expected_height,
                    "block received was stale because firm blocks were executed first; dropping",
                );
                return Ok(None);
            }
            std::cmp::Ordering::Greater => bail!(
                "block received was out-of-order; was a block skipped? expected: \
                 {expected_height}, actual: {}",
                executable_block.height
            ),
            std::cmp::Ordering::Equal => {}
        }

        let genesis_height = self.state.sequencer_start_block_height();
        let block_height = executable_block.height;
        let rollup_start_block_height = self.state.rollup_start_block_height();
        let Some(block_number) = state::map_sequencer_height_to_rollup_height(
            genesis_height,
            block_height,
            rollup_start_block_height,
        ) else {
            bail!(
                "failed to map block height rollup number. This means the operation
                `sequencer_height - sequencer_genesis_height` underflowed or was not a valid
                cometbft height. Sequencer height: `{block_height}`, sequencer genesis height: \
                 `{genesis_height}`",
            )
        };

        // The parent hash of the next block is the hash of the block at the current head.
        let parent_hash = self.state.soft_hash();
        let executed_block = self
            .execute_block(parent_hash, executable_block)
            .await
            .wrap_err("failed to execute block")?;

        self.does_block_response_fulfill_contract(ExecutionKind::Soft, &executed_block)
            .wrap_err("execution API server violated contract")?;

        self.update_commitment_state(Update::OnlySoft(executed_block.clone()))
            .await
            .wrap_err("failed to update soft commitment state")?;

        self.blocks_pending_finalization
            .insert(block_number, executed_block);

        // XXX: We set an absolute number value here to avoid any potential issues of the remote
        // rollup state and the local state falling out of lock-step.
        self.metrics
            .absolute_set_executed_soft_block_number(block_number);

        Ok(None)
    }

    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::base64(&block.block_hash),
        block.height = block.sequencer_height().value(),
        err,
    ))]
    async fn execute_firm(
        &mut self,
        block: ReconstructedBlock,
    ) -> eyre::Result<Option<StopHeightExceded>> {
        if self.is_firm_block_height_exceded(&block) {
            return Ok(Some(StopHeightExceded::celestia(
                block.header.height().value(),
                self.state.sequencer_stop_block_height(),
            )));
        }

        let celestia_height = block.celestia_height;
        let executable_block = ExecutableBlock::from_reconstructed(block);
        let expected_height = self.state.next_expected_firm_sequencer_height();
        let block_height = executable_block.height;
        let rollup_start_block_height = self.state.rollup_start_block_height();
        ensure!(
            block_height == expected_height,
            "expected block at sequencer height {expected_height}, but got {block_height}",
        );

        let genesis_height = self.state.sequencer_start_block_height();
        let Some(block_number) = state::map_sequencer_height_to_rollup_height(
            genesis_height,
            block_height,
            rollup_start_block_height,
        ) else {
            bail!(
                "failed to map block height rollup number. This means the operation
                `sequencer_height - sequencer_genesis_height` underflowed or was not a valid
                cometbft height. Sequencer height: `{block_height}`, sequencer genesis height: \
                 `{genesis_height}`",
            )
        };

        let update = if self.should_execute_firm_block() {
            let parent_hash = self.state.firm_hash();
            let executed_block = self
                .execute_block(parent_hash, executable_block)
                .await
                .wrap_err("failed to execute block")?;
            self.does_block_response_fulfill_contract(ExecutionKind::Firm, &executed_block)
                .wrap_err("execution API server violated contract")?;
            Update::ToSame(executed_block, celestia_height)
        } else if let Some(block) = self.blocks_pending_finalization.remove(&block_number) {
            debug!(
                block_number,
                "found pending block in cache; updating state but not not re-executing it"
            );
            Update::OnlyFirm(block, celestia_height)
        } else {
            debug!(
                block_number,
                "pending block not found for block number in cache. THIS SHOULD NOT HAPPEN. \
                 Trying to fetch the already-executed block from the rollup before giving up."
            );
            match self.client.get_block_with_retry(block_number).await {
                Ok(block) => Update::OnlyFirm(block, celestia_height),
                Err(error) => {
                    error!(
                        block_number,
                        %error,
                        "failed to fetch block from rollup and can will not be able to update \
                        firm commitment state. Giving up."
                    );
                    return Err(error).wrap_err_with(|| {
                        format!("failed to get block at number `{block_number}` from rollup")
                    });
                }
            }
        };

        self.update_commitment_state(update)
            .await
            .wrap_err("failed to setting both commitment states to executed block")?;

        // XXX: We set an absolute number value here to avoid any potential issues of the remote
        // rollup state and the local state falling out of lock-step.
        self.metrics
            .absolute_set_executed_soft_block_number(block_number);

        Ok(None)
    }

    /// Executes `block` on top of its `parent_hash`.
    ///
    /// This function is called via [`Executor::execute_firm`] or [`Executor::execute_soft`],
    /// and should not be called directly.
    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::base64(&block.hash),
        block.height = block.height.value(),
        block.num_of_transactions = block.transactions.len(),
        rollup.parent_hash = %telemetry::display::base64(&parent_hash),
        err
    ))]
    async fn execute_block(
        &mut self,
        parent_hash: Bytes,
        block: ExecutableBlock,
    ) -> eyre::Result<Block> {
        let ExecutableBlock {
            transactions,
            timestamp,
            ..
        } = block;

        let n_transactions = transactions.len();

        let executed_block = self
            .client
            .execute_block_with_retry(parent_hash, transactions, timestamp)
            .await
            .wrap_err("failed to run execute_block RPC")?;

        self.metrics
            .record_transactions_per_executed_block(n_transactions);

        info!(
            executed_block.hash = %telemetry::display::base64(&executed_block.hash()),
            executed_block.number = executed_block.number(),
            "executed block",
        );

        Ok(executed_block)
    }

    #[instrument(skip_all, err)]
    async fn set_initial_node_state(&mut self) -> eyre::Result<()> {
        let genesis_info = {
            async {
                self.client
                    .clone()
                    .get_genesis_info_with_retry()
                    .await
                    .wrap_err("failed getting genesis info")
            }
        };
        let commitment_state = {
            async {
                self.client
                    .clone()
                    .get_commitment_state_with_retry()
                    .await
                    .wrap_err("failed getting commitment state")
            }
        };
        let (genesis_info, commitment_state) = tokio::try_join!(genesis_info, commitment_state)?;
        self.state
            .try_init(genesis_info, commitment_state)
            .wrap_err("failed initializing state tracking")?;

        self.metrics
            .absolute_set_executed_firm_block_number(self.state.firm_number());
        self.metrics
            .absolute_set_executed_soft_block_number(self.state.soft_number());
        info!(
            initial_state = serde_json::to_string(&*self.state.get())
                .expect("writing json to a string should not fail"),
            "received genesis info from rollup",
        );
        Ok(())
    }

    #[instrument(skip_all, err)]
    async fn update_commitment_state(&mut self, update: Update) -> eyre::Result<()> {
        use Update::{
            OnlyFirm,
            OnlySoft,
            ToSame,
        };
        let (firm, soft, celestia_height) = match update {
            OnlyFirm(firm, celestia_height) => (firm, self.state.soft(), celestia_height),
            OnlySoft(soft) => (
                self.state.firm(),
                soft,
                self.state.celestia_base_block_height(),
            ),
            ToSame(block, celestia_height) => (block.clone(), block, celestia_height),
        };
        let commitment_state = CommitmentState::builder()
            .firm(firm)
            .soft(soft)
            .base_celestia_height(celestia_height)
            .build()
            .wrap_err("failed constructing commitment state")?;
        let new_state = self
            .client
            .update_commitment_state_with_retry(commitment_state)
            .await
            .wrap_err("failed updating remote commitment state")?;
        info!(
            soft.number = new_state.soft().number(),
            soft.hash = %telemetry::display::base64(&new_state.soft().hash()),
            firm.number = new_state.firm().number(),
            firm.hash = %telemetry::display::base64(&new_state.firm().hash()),
            "updated commitment state",
        );
        self.state
            .try_update_commitment_state(new_state)
            .wrap_err("failed updating internal state tracking rollup state; invalid?")?;
        Ok(())
    }

    fn does_block_response_fulfill_contract(
        &mut self,
        kind: ExecutionKind,
        block: &Block,
    ) -> Result<(), ContractViolation> {
        does_block_response_fulfill_contract(&mut self.state, kind, block)
    }

    /// Returns whether a firm block should be executed.
    ///
    /// Firm blocks should be executed if:
    /// 1. executor runs in firm only mode (blocks are always executed).
    /// 2. executor runs in soft-and-firm mode and the soft and firm rollup numbers are equal.
    fn should_execute_firm_block(&self) -> bool {
        should_execute_firm_block(
            self.state.next_expected_firm_sequencer_height().value(),
            self.state.next_expected_soft_sequencer_height().value(),
            self.mode,
        )
    }

    /// Returns whether the height of the soft block is greater than or equal to the stop block
    /// height.
    fn is_soft_block_height_exceded(&self, block: &ExecutableBlock) -> bool {
        let stop_height = self.state.sequencer_stop_block_height();
        stop_height > 0 && block.height.value() >= stop_height
    }

    /// Returns whether the height of the firm block is greater than the stop block height.
    fn is_firm_block_height_exceded(&self, block: &ReconstructedBlock) -> bool {
        let stop_height = self.state.sequencer_stop_block_height();
        stop_height > 0 && block.header.height().value() > stop_height
    }
}

#[instrument(skip_all)]
fn report_exit(reason: eyre::Result<ExitReason>, message: &str) -> eyre::Result<ExitReason> {
    match reason {
        Ok(reason) => {
            info!(%reason, message);
            Ok(reason)
        }
        Err(error) => {
            error!(%error, message);
            Err(error)
        }
    }
}

enum Update {
    OnlyFirm(Block, CelestiaHeight),
    OnlySoft(Block),
    ToSame(Block, CelestiaHeight),
}

#[derive(Debug)]
struct ExecutableBlock {
    hash: [u8; 32],
    height: SequencerHeight,
    timestamp: pbjson_types::Timestamp,
    transactions: Vec<Bytes>,
}

impl ExecutableBlock {
    fn from_reconstructed(block: ReconstructedBlock) -> Self {
        let ReconstructedBlock {
            block_hash,
            header,
            transactions,
            ..
        } = block;
        let timestamp = convert_tendermint_time_to_protobuf_timestamp(header.time());
        Self {
            hash: block_hash,
            height: header.height(),
            timestamp,
            transactions,
        }
    }

    fn from_sequencer(block: FilteredSequencerBlock, id: RollupId) -> Self {
        let FilteredSequencerBlockParts {
            block_hash,
            header,
            mut rollup_transactions,
            ..
        } = block.into_parts();
        let height = header.height();
        let timestamp = convert_tendermint_time_to_protobuf_timestamp(header.time());
        let transactions = rollup_transactions
            .swap_remove(&id)
            .map(|txs| txs.transactions().to_vec())
            .unwrap_or_default();
        Self {
            hash: block_hash,
            height,
            timestamp,
            transactions,
        }
    }
}

/// Converts a [`tendermint::Time`] to a [`prost_types::Timestamp`].
fn convert_tendermint_time_to_protobuf_timestamp(value: TendermintTime) -> pbjson_types::Timestamp {
    let sequencer_client::tendermint_proto::google::protobuf::Timestamp {
        seconds,
        nanos,
    } = value.into();
    pbjson_types::Timestamp {
        seconds,
        nanos,
    }
}

#[derive(Copy, Clone, Debug)]
enum ExecutionKind {
    Firm,
    Soft,
}

impl std::fmt::Display for ExecutionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            ExecutionKind::Firm => "firm",
            ExecutionKind::Soft => "soft",
        };
        f.write_str(kind)
    }
}

#[derive(Debug, thiserror::Error)]
enum ContractViolation {
    #[error(
        "contract violated: execution kind: {kind}, current block number {current}, expected \
         {expected}, received {actual}"
    )]
    WrongBlock {
        kind: ExecutionKind,
        current: u32,
        expected: u32,
        actual: u32,
    },
    #[error("contract violated: current height cannot be incremented")]
    CurrentBlockNumberIsMax { kind: ExecutionKind, actual: u32 },
}

fn does_block_response_fulfill_contract(
    state: &mut StateSender,
    kind: ExecutionKind,
    block: &Block,
) -> Result<(), ContractViolation> {
    let current = match kind {
        ExecutionKind::Firm => state.firm_number(),
        ExecutionKind::Soft => state.soft_number(),
    };
    let actual = block.number();
    let expected = current
        .checked_add(1)
        .ok_or(ContractViolation::CurrentBlockNumberIsMax {
            kind,
            actual,
        })?;
    if actual == expected {
        Ok(())
    } else {
        Err(ContractViolation::WrongBlock {
            kind,
            current,
            expected,
            actual,
        })
    }
}

fn should_execute_firm_block(
    firm_sequencer_height: u64,
    soft_sequencer_height: u64,
    executor_mode: CommitLevel,
) -> bool {
    match executor_mode {
        CommitLevel::SoftAndFirm if firm_sequencer_height == soft_sequencer_height => true,
        CommitLevel::SoftOnly | CommitLevel::SoftAndFirm => false,
        CommitLevel::FirmOnly => true,
    }
}
