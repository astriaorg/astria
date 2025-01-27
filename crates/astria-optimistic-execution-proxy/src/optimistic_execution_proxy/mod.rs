//! The Astria Auctioneer business logic.
use std::time::Duration;

use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::{
        optimistic::v1alpha1::SequencerBlockCommit,
        v1::block::FilteredSequencerBlock,
    },
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use futures::StreamExt as _;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    field,
    info,
    instrument,
    Span,
};

use crate::{
    rollup_channel::ExecuteOptimisticBlockStream,
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
    },
    Config,
};

/// The implementation of the auctioneer business logic.
pub(super) struct OptimisticExecutionProxy {
    block_commitments: BlockCommitmentStream,
    executed_blocks: ExecuteOptimisticBlockStream,
    optimistic_blocks: OptimisticBlockStream,
    rollup_id: RollupId,
    shutdown_token: CancellationToken,
}

impl OptimisticExecutionProxy {
    /// Creates an [`OptimisticExecutionProxy`] service from a [`Config`].
    pub(super) fn new(config: Config, shutdown_token: CancellationToken) -> eyre::Result<Self> {
        let Config {
            sequencer_grpc_endpoint,
            rollup_grpc_endpoint,
            rollup_id,
            ..
        } = config;

        let rollup_id = RollupId::from_unhashed_bytes(rollup_id);
        let rollup_channel = crate::rollup_channel::open(&rollup_grpc_endpoint)?;
        let sequencer_channel = crate::sequencer_channel::open(&sequencer_grpc_endpoint)?;

        Ok(Self {
            block_commitments: sequencer_channel.open_get_block_commitment_stream(),
            executed_blocks: rollup_channel.open_execute_optimistic_block_stream(),
            optimistic_blocks: sequencer_channel.open_get_optimistic_block_stream(rollup_id),
            rollup_id,
            shutdown_token,
        })
    }

    /// Runs the [`Auctioneer`] service until it received an exit signal, or one of the constituent
    /// tasks either ends unexpectedly or returns an error.
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let reason: eyre::Result<&str> = {
            // This is a long running loop. Errors are emitted inside the handlers.
            loop {
                select! {
                    biased;

                    () = self.shutdown_token.clone().cancelled_owned() => {
                        break Ok("received shutdown signal");
                    },

                    res = self.handle_streaming_event() => {
                        if let Err(err) = res {
                            break Err(err);
                        }
                    }
                }
            }
        };

        self.shutdown(reason).await
    }

    async fn handle_streaming_event(&mut self) -> eyre::Result<()> {
        select!(
            res = self.optimistic_blocks.next() => {
                let res = res.ok_or_eyre("optimistic block stream closed")?;
                let _ = self.handle_optimistic_block(res);
            },

            res = self.block_commitments.next() => {
                let res = res.ok_or_eyre("block commitment stream closed")?;
                let _ = self.handle_block_commitment(res);
            },

            res = self.executed_blocks.next() => {
                let res = res.ok_or_eyre("executed block stream closed")?;
                let _ = self.handle_executed_block(res);
            }
        );
        Ok(())
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_optimistic_block(
        &mut self,
        optimistic_block: eyre::Result<FilteredSequencerBlock>,
    ) -> eyre::Result<()> {
        let optimistic_block =
            optimistic_block.wrap_err("encountered problem receiving optimistic block message")?;

        Span::current().record("block_hash", field::display(optimistic_block.block_hash()));

        // TODO: do conversion && sending in one operation
        let base_block = crate::block::Optimistic::new(optimistic_block)
            .try_into_base_block(self.rollup_id)
            // FIXME: give this their proper wire names
            .wrap_err("failed to create BaseBlock from FilteredSequencerBlock")?;
        self.executed_blocks
            .try_send(base_block)
            .wrap_err("failed to forward block to execution stream")?;

        Ok(())
    }

    #[instrument(skip_all, fields(block_hash = field::Empty), err)]
    fn handle_block_commitment(
        &mut self,
        commitment: eyre::Result<SequencerBlockCommit>,
    ) -> eyre::Result<()> {
        let block_commitment = commitment.wrap_err("failed to receive block commitment")?;
        Span::current().record("block_hash", field::display(block_commitment.block_hash()));

        Ok(())
    }

    #[instrument(skip_all, fields(
        sequencer_block_hash = field::Empty,
        rollup_block_hash = field::Empty,
    ), err)]
    fn handle_executed_block(
        &mut self,
        executed_block: eyre::Result<crate::block::Executed>,
    ) -> eyre::Result<()> {
        let executed_block = executed_block.wrap_err("failed to receive executed block")?;
        Span::current().record(
            "sequencer_block_hash",
            field::display(executed_block.sequencer_block_hash()),
        );
        Span::current().record(
            "rollup_block_hash",
            field::display(executed_block.rollup_block_hash()),
        );
        Ok(())
    }

    #[instrument(skip_all)]
    async fn shutdown(self, reason: eyre::Result<&'static str>) -> eyre::Result<()> {
        const WAIT_BEFORE_ABORT: Duration = Duration::from_secs(25);

        // Necessary if we got here because of another reason than receiving an external
        // shutdown signal.
        self.shutdown_token.cancel();

        let message = format!(
            "waiting {} for all constituent tasks to shutdown before aborting",
            humantime::format_duration(WAIT_BEFORE_ABORT),
        );
        match &reason {
            Ok(reason) => info!(%reason, message),
            Err(reason) => error!(%reason, message),
        };
        reason.map(|_| ())
    }
}
