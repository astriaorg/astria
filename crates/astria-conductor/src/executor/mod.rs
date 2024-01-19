use std::collections::HashMap;

use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
    },
    sequencer::v1alpha1::RollupId,
};
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use sequencer_client::{
    tendermint::block::Height,
    SequencerBlock,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
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
pub(crate) mod optimism;

mod client;
#[cfg(test)]
mod tests;
mod track_state;
use track_state::TrackState;

pub(crate) struct Executor {
    celestia_rx: mpsc::UnboundedReceiver<ReconstructedBlock>,
    celestia_tx: mpsc::WeakUnboundedSender<ReconstructedBlock>,
    sequencer_rx: mpsc::UnboundedReceiver<SequencerBlock>,
    sequencer_tx: mpsc::WeakUnboundedSender<SequencerBlock>,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    client: client::Client,

    /// Chain ID
    rollup_id: RollupId,

    /// Tracks SOFT and FIRM on the execution chain
    commitment_state: TrackState,

    /// Tracks executed blocks as soft commitments.
    ///
    /// Required to mark firm blocks received from celestia as executed
    /// without re-executing on top of the rollup node on top of the rollup node..
    blocks_pending_finalization: HashMap<[u8; 32], Block>,

    /// optional hook which is called to modify the rollup transaction list
    /// right before it's sent to the execution layer via `ExecuteBlock`.
    pre_execution_hook: Option<optimism::Handler>,
}

impl Executor {
    pub(super) fn builder() -> builder::ExecutorBuilder {
        builder::ExecutorBuilder::new()
    }

    /// Returns the next sequencer height expected by the executor.
    pub(super) fn next_soft_sequencer_height(&self) -> Height {
        self.commitment_state.next_soft_sequencer_height()
    }

    pub(super) fn celestia_channel(&self) -> mpsc::UnboundedSender<ReconstructedBlock> {
        self.celestia_tx.upgrade().expect(
            "should work because the channel is held by self, is open, and other senders exist",
        )
    }

    pub(super) fn sequencer_channel(&self) -> mpsc::UnboundedSender<SequencerBlock> {
        self.sequencer_tx.upgrade().expect(
            "should work because the channel is held by self, is open, and other senders exist",
        )
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        loop {
            select!(
                biased;

                shutdown = &mut self.shutdown => {
                    let ret = if let Err(e) = shutdown {
                        let reason = "shutdown channel closed unexpectedly";
                        error!(error = &e as &dyn std::error::Error, reason, "shutting down");
                        Err(e).wrap_err(reason)
                    } else {
                        info!(reason = "received shutdown signal", "shutting down");
                        Ok(())
                    };
                    break ret;
                }

                Some(block) = self.celestia_rx.recv() => {
                    debug!(
                        block.height = %block.height(),
                        block.hash = %telemetry::display::hex(&block.block_hash),
                        "received block from celestia reader",
                    );
                    if let Err(e) = self.execute_firm(block).await {
                        let reason = "failed executing firm block";
                        error!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            reason,
                            "shutting down",
                        );
                        break Err(e).wrap_err(reason);
                    }
                }

                Some(block) = self.sequencer_rx.recv() => {
                    debug!(
                        block.height = %block.height(),
                        block.hash = %telemetry::display::hex(&block.block_hash()),
                        "received block from sequencer reader",
                    );
                    if let Err(e) = self.execute_soft(block).await {
                        let reason = "failed executing soft block";
                        error!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            reason,
                            "shutting down",
                        );
                        break Err(e).wrap_err(reason);
                    }
                }
            );
        }
        // XXX: shut down the channels here and attempt to drain them before returning.
    }

    #[instrument(skip_all, fields(
        hash.sequencer_block = %telemetry::display::hex(&block.block_hash()),
        height.sequencer_block = %block.height(),
    ))]
    async fn execute_soft(&mut self, block: SequencerBlock) -> eyre::Result<()> {
        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        let executable_block = ExecutableBlock::from_sequencer(block, self.rollup_id);

        match executable_block
            .height
            .cmp(&self.commitment_state.next_soft_sequencer_height())
        {
            std::cmp::Ordering::Less => {
                // XXX: we don't track if older sequencer blocks are sequential, only if they are
                // newer (`Greater` arm)
                info!(
                    expected_height.sequencer_block = %self.commitment_state.next_soft_sequencer_height(),
                    "block received was at at older height or stale because firm blocks were executed first; dropping",
                );
                return Ok(());
            }
            std::cmp::Ordering::Greater => bail!(
                "block received was out-of-order; was a block skipped? expected: {}, actual: {}",
                self.commitment_state.next_soft_sequencer_height(),
                executable_block.height
            ),
            std::cmp::Ordering::Equal => {}
        }

        let block_hash = executable_block.hash;

        let parent_block_hash = self.commitment_state.soft_parent_hash();
        let executed_block = self
            .execute_block(parent_block_hash, executable_block)
            .await
            .wrap_err("failed to execute block")?;

        self.update_commitment_state(Update::OnlySoft(executed_block.clone()))
            .await
            .wrap_err("failed to update soft commitment state")?;

        self.blocks_pending_finalization
            .insert(block_hash, executed_block);

        Ok(())
    }

    async fn execute_firm(&mut self, block: ReconstructedBlock) -> eyre::Result<()> {
        let executable_block = ExecutableBlock::from_reconstructed(block);
        ensure!(
            executable_block.height == self.commitment_state.next_firm_sequencer_height(),
            "expected block at sequencer height {}, but got {}",
            self.commitment_state.next_firm_sequencer_height(),
            executable_block.height,
        );

        if let Some(block) = self
            .blocks_pending_finalization
            .remove(&executable_block.hash)
        {
            self.update_commitment_state(Update::OnlyFirm(block))
                .await
                .wrap_err("failed to update firm commitment state")?;
        } else {
            let parent_block_hash = self.commitment_state.firm_parent_hash();
            let executed_block = self
                .execute_block(parent_block_hash, executable_block)
                .await
                .wrap_err("failed to execute block")?;

            self.update_commitment_state(Update::ToSame(executed_block))
                .await
                .wrap_err("failed to setting both commitment states to executed block")?;
        }
        Ok(())
    }

    /// Executes `block` on top of its `parent_block_hash`.
    ///
    /// This function is called via [`Executor::execute_firm`] or [`Executor::execute_soft`],
    /// and should not be called directly.
    #[instrument(skip_all, fields(
        hash.sequencer_block = %telemetry::display::hex(&block.hash),
        height.sequencer_block = %block.height,
        hash.parent_block = %telemetry::display::hex(&parent_block_hash),
    ))]
    async fn execute_block(
        &mut self,
        parent_block_hash: [u8; 32],
        block: ExecutableBlock,
    ) -> eyre::Result<Block> {
        let ExecutableBlock {
            mut transactions,
            timestamp,
            ..
        } = block;

        if let Some(hook) = self.pre_execution_hook.as_mut() {
            transactions = hook
                .populate_rollup_transactions(transactions)
                .await
                .wrap_err("failed to populate rollup transactions with pre execution hook")?;
        }

        let executed_block = self
            .client
            .execute_block(parent_block_hash, transactions, timestamp)
            .await
            .wrap_err("failed to run execute_block RPC")?;

        debug!(
            hash.executed_block = %telemetry::display::hex(&executed_block.hash()),
            height.executed_block = executed_block.number(),
            "executed block",
        );

        Ok(executed_block)
    }

    async fn update_commitment_state(&mut self, update: Update) -> eyre::Result<()> {
        use Update::{
            OnlyFirm,
            OnlySoft,
            ToSame,
        };
        let (firm, soft) = match update {
            OnlyFirm(firm) => (firm, self.commitment_state.soft().clone()),
            OnlySoft(soft) => (self.commitment_state.firm().clone(), soft),
            ToSame(block) => (block.clone(), block),
        };
        let commitment_state = CommitmentState::builder()
            .firm(firm)
            .soft(soft)
            .build()
            .wrap_err("failed constructing commitment state")?;
        let new_state = self
            .client
            .update_commitment_state(commitment_state)
            .await
            .wrap_err("failed updating remote commitment state")?;
        self.commitment_state.set_state(new_state);
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
    height: Height,
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
        let timestamp = convert_tendermint_to_prost_timestamp(block.header().time);
        let transactions = block
            .into_rollup_transactions()
            .remove(&id)
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
fn convert_tendermint_to_prost_timestamp(value: tendermint::Time) -> prost_types::Timestamp {
    let tendermint_proto::google::protobuf::Timestamp {
        seconds,
        nanos,
    } = value.into();
    prost_types::Timestamp {
        seconds,
        nanos,
    }
}
