use std::collections::HashMap;

use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
    },
    sequencer::v1alpha1::RollupId,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use bytes::Bytes;
use celestia_client::celestia_types::Height as CelestiaHeight;
use sequencer_client::{
    tendermint::{
        block::Height as SequencerHeight,
        Time,
    },
    SequencerBlock,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            error::{
                SendError,
                TrySendError,
            },
        },
        oneshot,
        watch::{
            self,
            error::RecvError,
        },
    },
};
use tracing::{
    debug,
    error,
    info,
    instrument,
};

use crate::celestia::ReconstructedBlock;

mod builder;
pub(crate) mod channel;

use channel::soft_block_channel;

mod client;
mod state;
#[cfg(test)]
mod tests;

pub(super) use client::Client;
pub(crate) use state::State;

#[derive(Clone, Debug)]
pub(crate) struct StateNotInit;
#[derive(Clone, Debug)]
pub(crate) struct StateIsInit;

/// A handle to the executor.
///
/// To be be useful, [`Handle<StateNotInit>::wait_for_init`] must be called in
/// order to obtain a [`Handle<StateInit>`]. This is to ensure that the executor
/// state was primed before using its other methods. See [`State`] for more
/// information.
#[derive(Debug, Clone)]
pub(crate) struct Handle<TStateInit = StateNotInit> {
    firm_blocks: mpsc::Sender<ReconstructedBlock>,
    soft_blocks: channel::Sender<SequencerBlock>,
    state: watch::Receiver<State>,
    _state_init: TStateInit,
}

impl<T: Clone> Handle<T> {
    #[instrument(skip_all, err)]
    pub(crate) async fn wait_for_init(&mut self) -> eyre::Result<Handle<StateIsInit>> {
        self.state
            .wait_for(State::is_init)
            .await
            .wrap_err("executor state channel terminated before initial state could be observed")?;
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
    pub(crate) async fn send_firm_block(
        self,
        block: ReconstructedBlock,
    ) -> Result<(), SendError<ReconstructedBlock>> {
        self.firm_blocks.send(block).await
    }

    // allow: return value of tokio's mpsc send try_send method
    #[allow(clippy::result_large_err)]
    pub(crate) fn try_send_firm_block(
        &self,
        block: ReconstructedBlock,
    ) -> Result<(), TrySendError<ReconstructedBlock>> {
        self.firm_blocks.try_send(block)
    }

    pub(crate) async fn send_soft_block_owned(
        self,
        block: SequencerBlock,
    ) -> Result<(), channel::SendError> {
        self.soft_blocks.send(block).await
    }

    // allow: this is mimicking tokio's `SendError` that returns the stack-allocated object.
    #[allow(clippy::result_large_err)]
    pub(crate) fn try_send_soft_block(
        &self,
        block: SequencerBlock,
    ) -> Result<(), channel::TrySendError<SequencerBlock>> {
        self.soft_blocks.try_send(block)
    }

    pub(crate) fn next_expected_firm_height(&mut self) -> SequencerHeight {
        self.state.borrow_and_update().next_firm_sequencer_height()
    }

    pub(crate) fn next_expected_soft_height(&mut self) -> SequencerHeight {
        self.state.borrow_and_update().next_soft_sequencer_height()
    }

    pub(crate) async fn next_expected_soft_height_if_changed(
        &mut self,
    ) -> Result<SequencerHeight, RecvError> {
        self.state.changed().await?;
        Ok(self.state.borrow_and_update().next_soft_sequencer_height())
    }

    pub(crate) fn rollup_id(&mut self) -> RollupId {
        self.state.borrow_and_update().rollup_id()
    }

    pub(crate) fn celestia_base_block_height(&mut self) -> CelestiaHeight {
        self.state.borrow_and_update().celestia_base_block_height()
    }

    pub(crate) fn celestia_block_variance(&mut self) -> u32 {
        self.state.borrow_and_update().celestia_block_variance()
    }
}

pub(crate) struct Executor {
    firm_blocks: mpsc::Receiver<ReconstructedBlock>,
    soft_blocks: channel::Receiver<SequencerBlock>,

    shutdown: oneshot::Receiver<()>,

    rollup_address: tonic::transport::Uri,

    /// Tracks SOFT and FIRM on the execution chain
    state: watch::Sender<State>,

    // If set, the executor will take into account the spread between firm
    // and soft commitments when executing blocks.
    consider_commitment_spread: bool,

    /// Tracks executed blocks as soft commitments.
    ///
    /// Required to mark firm blocks received from celestia as executed
    /// without re-executing on top of the rollup node on top of the rollup node..
    blocks_pending_finalization: HashMap<[u8; 32], Block>,
}

impl Executor {
    pub(super) fn builder() -> builder::ExecutorBuilder {
        builder::ExecutorBuilder::new()
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let client = Client::connect(self.rollup_address.clone())
            .await
            .wrap_err("failed connecting to rollup node")?;

        self.set_initial_node_state(client.clone())
            .await
            .wrap_err("failed setting initial rollup node state")?;

        let max_spread: usize = self.calculate_max_spread();
        self.soft_blocks.set_capacity(max_spread);

        info!(
            max_spread,
            "setting capacity of soft blocks channel to maximum permitted firm<>soft commitment \
             spread (this has no effect if conductor is set to perform soft-sync only)"
        );

        loop {
            let spread_not_too_large = !self.is_spread_too_large(max_spread);
            if spread_not_too_large {
                self.soft_blocks.fill_permits();
            }

            select!(
                biased;

                shutdown = &mut self.shutdown => {
                    let ret = if let Err(error) = shutdown {
                        let reason = "shutdown channel closed unexpectedly";
                        error!(%error, reason, "shutting down");
                        Err(error).wrap_err(reason)
                    } else {
                        info!(reason = "received shutdown signal", "shutting down");
                        Ok(())
                    };
                    break ret;
                }

                Some(block) = self.firm_blocks.recv() => {
                    debug!(
                        block.height = %block.height(),
                        block.hash = %telemetry::display::hex(&block.block_hash),
                        "received block from celestia reader",
                    );
                    if let Err(error) = self.execute_firm(client.clone(), block).await {
                        let reason = "failed executing firm block";
                        error!(%error, reason, "shutting down");
                        break Err(error).wrap_err(reason);
                    }
                }

                Some(block) = self.soft_blocks.recv(), if spread_not_too_large => {
                    debug!(
                        block.height = %block.height(),
                        block.hash = %telemetry::display::hex(&block.block_hash()),
                        "received block from sequencer reader",
                    );
                    if let Err(error) = self.execute_soft(client.clone(), block).await {
                        let reason = "failed executing soft block";
                        error!(%error, reason, "shutting down");
                        break Err(error).wrap_err(reason);
                    }
                }
            );
        }
        // XXX: shut down the channels here and attempt to drain them before returning.
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
        usize::try_from(self.state.borrow().celestia_block_variance())
            .expect("converting a u32 to usize should work on any architecture conductor runs on")
            .saturating_mul(6)
    }

    /// Returns if the spread between firm and soft commitment heights in the tracked state is too
    /// large.
    ///
    /// Always returns `false` if this executor was configured with `consider_commitment_spread =
    /// false`.
    fn is_spread_too_large(&self, max_spread: usize) -> bool {
        if !self.consider_commitment_spread {
            return false;
        }
        let (next_firm, next_soft) = {
            let state = self.state.borrow();
            let next_firm = state.next_firm_sequencer_height().value();
            let next_soft = state.next_soft_sequencer_height().value();
            (next_firm, next_soft)
        };

        let is_too_far_ahead = usize::try_from(next_soft.saturating_sub(next_firm))
            .map(|spread| spread >= max_spread)
            .unwrap_or(false);

        if is_too_far_ahead {
            debug!("soft blocks are too far ahead of firm; skipping soft blocks");
        }
        is_too_far_ahead
    }

    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::hex(&block.block_hash()),
        block.height = block.height().value(),
    ))]
    async fn execute_soft(&mut self, client: Client, block: SequencerBlock) -> eyre::Result<()> {
        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        let executable_block =
            ExecutableBlock::from_sequencer(block, self.state.borrow().rollup_id());

        let expected_height = self.state.borrow().next_soft_sequencer_height();
        match executable_block.height.cmp(&expected_height) {
            std::cmp::Ordering::Less => {
                info!(
                    expected_height.sequencer_block = %expected_height,
                    "block received was stale because firm blocks were executed first; dropping",
                );
                return Ok(());
            }
            std::cmp::Ordering::Greater => bail!(
                "block received was out-of-order; was a block skipped? expected: \
                 {expected_height}, actual: {}",
                executable_block.height
            ),
            std::cmp::Ordering::Equal => {}
        }

        let block_hash = executable_block.hash;

        let parent_block_hash = self.state.borrow().soft_parent_hash();
        let executed_block = self
            .execute_block(client.clone(), parent_block_hash, executable_block)
            .await
            .wrap_err("failed to execute block")?;

        does_block_response_fulfill_contract(
            ExecutionKind::Soft,
            &self.state.borrow(),
            &executed_block,
        )
        .wrap_err("execution API server violated contract")?;

        self.update_commitment_state(client.clone(), Update::OnlySoft(executed_block.clone()))
            .await
            .wrap_err("failed to update soft commitment state")?;

        self.blocks_pending_finalization
            .insert(block_hash, executed_block);

        Ok(())
    }

    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::hex(&block.block_hash),
        block.height = block.height().value(),
    ))]
    async fn execute_firm(
        &mut self,
        client: Client,
        block: ReconstructedBlock,
    ) -> eyre::Result<()> {
        let executable_block = ExecutableBlock::from_reconstructed(block);
        let expected_height = self.state.borrow().next_firm_sequencer_height();
        ensure!(
            executable_block.height == expected_height,
            "expected block at sequencer height {expected_height}, but got {}",
            executable_block.height,
        );

        let update_type = if let Some(block) = self
            .blocks_pending_finalization
            .remove(&executable_block.hash)
        {
            Update::OnlyFirm(block)
        } else {
            let parent_block_hash = self.state.borrow().firm_parent_hash();
            let executed_block = self
                .execute_block(client.clone(), parent_block_hash, executable_block)
                .await
                .wrap_err("failed to execute block")?;
            does_block_response_fulfill_contract(
                ExecutionKind::Firm,
                &self.state.borrow(),
                &executed_block,
            )
            .wrap_err("execution API server violated contract")?;
            Update::ToSame(executed_block)
        };
        self.update_commitment_state(client.clone(), update_type)
            .await
            .wrap_err("failed to setting both commitment states to executed block")?;
        Ok(())
    }

    /// Executes `block` on top of its `parent_block_hash`.
    ///
    /// This function is called via [`Executor::execute_firm`] or [`Executor::execute_soft`],
    /// and should not be called directly.
    #[instrument(skip_all, fields(
        block.hash = %telemetry::display::hex(&block.hash),
        block.height = block.height.value(),
        block.num_of_transactions = block.transactions.len(),
        rollup.parent_hash = %telemetry::display::hex(&parent_block_hash),
    ))]
    async fn execute_block(
        &mut self,
        mut client: Client,
        parent_block_hash: Bytes,
        block: ExecutableBlock,
    ) -> eyre::Result<Block> {
        let ExecutableBlock {
            transactions,
            timestamp,
            ..
        } = block;

        let executed_block = client
            .execute_block(parent_block_hash, transactions, timestamp)
            .await
            .wrap_err("failed to run execute_block RPC")?;

        info!(
            executed_block.hash = %telemetry::display::hex(&executed_block.hash()),
            executed_block.number = executed_block.number(),
            "executed block",
        );

        Ok(executed_block)
    }

    #[instrument(skip_all)]
    async fn set_initial_node_state(&self, client: Client) -> eyre::Result<()> {
        let genesis_info = {
            let mut client = client.clone();
            async move {
                client
                    .get_genesis_info()
                    .await
                    .wrap_err("failed getting genesis info")
            }
        };
        let commitment_state = {
            let mut client = client.clone();
            async move {
                client
                    .get_commitment_state()
                    .await
                    .wrap_err("failed getting commitment state")
            }
        };
        let (genesis_info, commitment_state) = tokio::try_join!(genesis_info, commitment_state)?;
        self.state
            .send_modify(move |state| state.init(genesis_info, commitment_state));
        info!(
            initial_state = serde_json::to_string(&*self.state.borrow())
                .expect("writing json to a string should not fail"),
            "received genesis info from rollup",
        );
        Ok(())
    }

    #[instrument(skip_all)]
    async fn update_commitment_state(
        &mut self,
        mut client: Client,
        update: Update,
    ) -> eyre::Result<()> {
        use Update::{
            OnlyFirm,
            OnlySoft,
            ToSame,
        };
        let (firm, soft) = match update {
            OnlyFirm(firm) => (firm, self.state.borrow().soft().clone()),
            OnlySoft(soft) => (self.state.borrow().firm().clone(), soft),
            ToSame(block) => (block.clone(), block),
        };
        let commitment_state = CommitmentState::builder()
            .firm(firm)
            .soft(soft)
            .build()
            .wrap_err("failed constructing commitment state")?;
        let new_state = client
            .update_commitment_state(commitment_state)
            .await
            .wrap_err("failed updating remote commitment state")?;
        info!(
            soft.number = new_state.soft().number(),
            soft.hash = %telemetry::display::hex(&new_state.soft().hash()),
            firm.number = new_state.firm().number(),
            firm.hash = %telemetry::display::hex(&new_state.firm().hash()),
            "updated commitment state",
        );
        self.state
            .send_if_modified(move |state| state.update_if_modified(new_state));
        Ok(())
    }
}

enum Update {
    OnlyFirm(Block),
    OnlySoft(Block),
    ToSame(Block),
}

#[derive(Debug)]
struct ExecutableBlock {
    hash: [u8; 32],
    height: SequencerHeight,
    timestamp: prost_types::Timestamp,
    transactions: Vec<Vec<u8>>,
}

impl ExecutableBlock {
    fn from_reconstructed(block: ReconstructedBlock) -> Self {
        let ReconstructedBlock {
            block_hash,
            header,
            transactions,
        } = block;
        let timestamp = convert_tendermint_to_prost_timestamp(header.time);
        Self {
            hash: block_hash,
            height: header.height,
            timestamp,
            transactions,
        }
    }

    fn from_sequencer(block: SequencerBlock, id: RollupId) -> Self {
        let hash = block.block_hash();
        let height = block.height();
        let timestamp = convert_tendermint_to_prost_timestamp(block.header().header().time);
        let transactions = block
            .into_rollup_transactions()
            .swap_remove(&id)
            .map(|txs| txs.transactions().to_vec())
            .unwrap_or_default();
        Self {
            hash,
            height,
            timestamp,
            transactions,
        }
    }
}

/// Converts a [`tendermint::Time`] to a [`prost_types::Timestamp`].
fn convert_tendermint_to_prost_timestamp(value: Time) -> prost_types::Timestamp {
    let sequencer_client::tendermint_proto::google::protobuf::Timestamp {
        seconds,
        nanos,
    } = value.into();
    prost_types::Timestamp {
        seconds,
        nanos,
    }
}

#[derive(Debug)]
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
#[error(
    "contract violated: execution kind: {kind}, current block number {current}, expected \
     {expected}, received {actual}"
)]
struct ContractViolation {
    kind: ExecutionKind,
    current: u32,
    expected: u32,
    actual: u32,
}

fn does_block_response_fulfill_contract(
    kind: ExecutionKind,
    state: &State,
    block: &Block,
) -> Result<(), ContractViolation> {
    let current = match kind {
        ExecutionKind::Firm => state.firm().number(),
        ExecutionKind::Soft => state.soft().number(),
    };
    let expected = current + 1;
    let actual = block.number();
    if actual == expected {
        Ok(())
    } else {
        Err(ContractViolation {
            kind,
            current,
            expected,
            actual,
        })
    }
}
