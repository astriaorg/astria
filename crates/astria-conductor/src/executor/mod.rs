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
use prost_types::Timestamp as ProstTimestamp;
use sequencer_client::SequencerBlock;
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

// Given `Time`, convert to protobuf timestamp
fn convert_tendermint_to_prost_timestamp(value: tendermint::Time) -> ProstTimestamp {
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

pub(crate) struct Executor {
    celestia_rx: mpsc::UnboundedReceiver<ReconstructedBlock>,
    celestia_tx: mpsc::WeakUnboundedSender<ReconstructedBlock>,
    sequencer_rx: mpsc::UnboundedReceiver<Box<SequencerBlock>>,
    sequencer_tx: mpsc::WeakUnboundedSender<Box<SequencerBlock>>,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    client: client::Client,

    /// Chain ID
    rollup_id: RollupId,

    /// Tracks SOFT and FIRM on the execution chain
    commitment_state: CommitmentState,

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
    sequencer_hash_to_execution_block: HashMap<[u8; 32], Block>,

    /// optional hook which is called to modify the rollup transaction list
    /// right before it's sent to the execution layer via `ExecuteBlock`.
    pre_execution_hook: Option<optimism::Handler>,
}

impl Executor {
    pub(super) fn builder() -> builder::ExecutorBuilder {
        builder::ExecutorBuilder::new()
    }

    pub(super) fn celestia_channel(&self) -> mpsc::UnboundedSender<ReconstructedBlock> {
        self.celestia_tx.upgrade().expect(
            "should work because the channel is held by self, is open, and other senders exist",
        )
    }

    pub(super) fn sequencer_channel(&self) -> mpsc::UnboundedSender<Box<SequencerBlock>> {
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
                        let message = "shutdown channel closed unexpectedly";
                        error!(error = &e as &dyn std::error::Error, "{message}, shutting down");
                        Err(e).wrap_err(message)
                    } else {
                        info!("received_shutdown_signal, shutting down");
                        Ok(())
                    };
                    break ret;
                }

                Some(block) = self.celestia_rx.recv() => {
                    if let Err(e) = self.execute_firm_block(block).await {
                        let message = "failed finalizing celestia blocks";
                        error!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            "{message}, shutting down",
                        );
                        break Err(e).wrap_err(message);
                    }
                }

                Some(block) = self.sequencer_rx.recv() => {
                    if let Err(e) = self.execute_soft_block(block).await {
                        let message = "failed executing block from sequencer";
                        error!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            "{message}, shutting down",
                        );
                        break Err(e).wrap_err(message);
                    }
                }
            );
        }
    }

    async fn execute_soft_block(&mut self, block: Box<SequencerBlock>) -> eyre::Result<()> {
        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        // XXX: this will be replaced.
        let block_hash = block.block_hash();
        let mut block = (*block).into_unchecked();
        let reconstructed_block = ReconstructedBlock {
            block_hash,
            header: block.header,
            transactions: block
                .rollup_transactions
                .remove(&self.rollup_id)
                .unwrap_or_default(),
        };

        let executed_block = self
            .execute_block(reconstructed_block)
            .await
            .wrap_err("failed to execute block")?;
        self.update_commitment_state(Update::OnlySoft(executed_block))
            .await
            .wrap_err("failed to update soft commitment state")?;
        Ok(())
    }

    /// Execute the sequencer block on the execution layer, returning the
    /// resulting execution block. If the block has already been executed, it
    /// returns the previously-computed execution block hash.
    #[instrument(skip(self), fields(sequencer_block_hash = ?block.block_hash, sequencer_block_height = block.header.height.value()))]
    async fn execute_block(&mut self, block: ReconstructedBlock) -> eyre::Result<Block> {
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
                execution_hash = hex::encode(execution_block.hash()),
                "block already executed"
            );
            return Ok(execution_block.clone());
        }

        let prev_block_hash = self.commitment_state.soft().hash();
        info!(
            sequencer_block_height = block.header.height.value(),
            parent_block_hash = hex::encode(prev_block_hash),
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
            .client
            .execute_block(prev_block_hash, rollup_transactions, timestamp)
            .await
            .wrap_err("failed to call execute_block")?;

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            execution_block_hash = %telemetry::display::hex(&executed_block.hash()),
            tx_count,
            "executed sequencer block",
        );

        // store block returned by execution client, as we need it to finalize the block later
        self.sequencer_hash_to_execution_block
            .insert(block.block_hash, executed_block.clone());

        Ok(executed_block)
    }

    async fn execute_firm_block(&mut self, block: ReconstructedBlock) -> eyre::Result<()> {
        let finalizable_block_height = self
            .calculate_finalizable_block_height()
            .wrap_err("failed calculating next finalizable block height")?;
        if block.header.height < finalizable_block_height {
            info!(
                sequencer_block_height = %block.header.height,
                finalized_block_height = %finalizable_block_height,
                "received block which is already finalized; skipping finalization"
            );
            return Ok(());
        }

        let sequencer_block_hash = block.block_hash;
        let maybe_executed_block = self
            .sequencer_hash_to_execution_block
            .get(&sequencer_block_hash)
            .cloned();
        if let Some(block) = maybe_executed_block {
            // this case means block has already been executed.
            self.update_commitment_state(Update::OnlyFirm(block))
                .await
                .wrap_err("failed to update firm commitment state")?;
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
            self.update_commitment_state(Update::ToSame(executed_block))
                .await
                .wrap_err("failed to setting both commitment states to executed block")?;
            // remove the sequencer block hash from the map, as it's been firmly committed
            self.sequencer_hash_to_execution_block
                .remove(&sequencer_block_hash);
        };
        Ok(())
    }

    // Returns the next sequencer block height which can be executed on the rollup
    pub(crate) fn calculate_executable_block_height(
        &self,
    ) -> eyre::Result<tendermint::block::Height> {
        let Some(executable_block_height) = calculate_sequencer_block_height(
            self.sequencer_height_with_first_rollup_block,
            self.commitment_state.soft().number(),
        ) else {
            bail!(
                "encountered overflow when calculating executable block height; sequencer height \
                 with first rollup block: {}, height recorded in soft commitment state: {}",
                self.sequencer_height_with_first_rollup_block,
                self.commitment_state.soft().number(),
            );
        };

        Ok(executable_block_height.into())
    }

    // Returns the lowest sequencer block height which can finalized on the rollup.
    pub(crate) fn calculate_finalizable_block_height(
        &self,
    ) -> eyre::Result<tendermint::block::Height> {
        let Some(finalizable_block_height) = calculate_sequencer_block_height(
            self.sequencer_height_with_first_rollup_block,
            self.commitment_state.firm().number(),
        ) else {
            bail!(
                "encountered overflow when calculating finalizable block height; sequencer height \
                 with first rollup block: {}, height recorded in firm commitment state: {}",
                self.sequencer_height_with_first_rollup_block,
                self.commitment_state.firm().number(),
            );
        };

        Ok(finalizable_block_height.into())
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
        self.commitment_state = new_state;
        Ok(())
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

enum Update {
    OnlyFirm(Block),
    OnlySoft(Block),
    ToSame(Block),
}
