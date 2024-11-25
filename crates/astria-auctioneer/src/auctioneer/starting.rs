use std::time::Duration;

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use futures::StreamExt as _;
use tokio::select;
use tokio_util::sync::CancellationToken;

use super::{
    inner::RunState,
    running::Running,
};
use crate::{
    rollup_channel::{
        BundleStream,
        ExecuteOptimisticBlockStream,
        RollupChannel,
    },
    sequencer_channel::{
        BlockCommitmentStream,
        OptimisticBlockStream,
        SequencerChannel,
    },
    Config,
    Metrics,
};

pub(super) fn run_state(
    config: Config,
    shutdown_token: CancellationToken,
    metrics: &'static Metrics,
) -> eyre::Result<RunState> {
    Starting::new(config, shutdown_token, metrics).map(Into::into)
}

pub(super) struct Starting {
    auctions: crate::auction::Manager,
    rollup_channel: RollupChannel,
    rollup_id: RollupId,
    sequencer_channel: SequencerChannel,
    shutdown_token: CancellationToken,
}

impl Starting {
    fn new(
        config: Config,
        shutdown_token: CancellationToken,
        metrics: &'static Metrics,
    ) -> eyre::Result<Self> {
        let Config {
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            latency_margin_ms,
            rollup_grpc_endpoint,
            rollup_id,
            sequencer_chain_id,
            sequencer_private_key_path,
            sequencer_address_prefix,
            fee_asset_denomination,
            ..
        } = config;

        let rollup_channel = crate::rollup_channel::open(&rollup_grpc_endpoint)?;
        let sequencer_channel = crate::sequencer_channel::open(&sequencer_grpc_endpoint)?;

        // TODO: Rearchitect this thing
        let auctions = crate::auction::manager::Builder {
            metrics,
            shutdown_token: shutdown_token.clone(),
            sequencer_grpc_endpoint: sequencer_grpc_endpoint.clone(),
            sequencer_abci_endpoint,
            latency_margin: Duration::from_millis(latency_margin_ms),
            sequencer_private_key_path,
            sequencer_address_prefix,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id: rollup_id.clone(),
        }
        .build()
        .wrap_err("failed to initialize auction manager")?;

        Ok(Starting {
            auctions,
            rollup_channel,
            rollup_id: RollupId::from_unhashed_bytes(&rollup_id),
            sequencer_channel,
            shutdown_token,
        })
    }

    pub(super) async fn run(self) -> eyre::Result<RunState> {
        select!(
            biased;

            () = self.shutdown_token.clone().cancelled_owned() => Ok(RunState::Cancelled),
            res = self.start_running() => res,
        )
    }

    async fn start_running(self) -> eyre::Result<RunState> {
        let Self {
            auctions,
            rollup_channel,
            rollup_id,
            sequencer_channel,
            shutdown_token,
        } = self;
        let (bundles, executed_blocks, block_commitments, (optimistic_blocks, current_block)) = tokio::try_join!(
            open_bundle_stream(rollup_channel.clone()),
            open_execute_optimistic_block_stream(rollup_channel.clone()),
            open_block_commitment_stream(sequencer_channel.clone()),
            open_optimistic_block_stream_and_get_current_block(
                sequencer_channel.clone(),
                rollup_id
            ),
        )?;
        Ok(Running {
            auctions,
            block_commitments,
            bundles,
            current_block,
            executed_blocks,
            optimistic_blocks,
            rollup_id,
            shutdown_token,
        }
        .into())
    }
}

async fn open_optimistic_block_stream_and_get_current_block(
    chan: SequencerChannel,
    rollup_id: RollupId,
) -> eyre::Result<(OptimisticBlockStream, crate::block::Current)> {
    let mut the_stream = chan
        .open_get_optimistic_block_stream(rollup_id)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to open optimistic block stream to Sequencer node for rollup ID \
                 `{rollup_id}`"
            )
        })?;
    let optimistic_block = the_stream
        .next()
        .await
        .ok_or_eyre("optimistic block stream closed before yielding the current block")?
        .wrap_err(
            "failed to get current optimistic block after opening a stream to the Sequencer node",
        )?;
    let current_block = crate::block::Current::with_optimistic(optimistic_block);
    Ok((the_stream, current_block))
}

async fn open_block_commitment_stream(
    chan: SequencerChannel,
) -> eyre::Result<BlockCommitmentStream> {
    chan.open_get_block_commitment_stream()
        .await
        .wrap_err("failed to open block commitment stream to sequencer node")
}

async fn open_bundle_stream(chan: RollupChannel) -> eyre::Result<BundleStream> {
    chan.open_bundle_stream()
        .await
        .wrap_err("failed to open `bundle stream` to rollup node")
}
async fn open_execute_optimistic_block_stream(
    chan: RollupChannel,
) -> eyre::Result<ExecuteOptimisticBlockStream> {
    chan.open_execute_optimistic_block_stream()
        .await
        .wrap_err("failed to open `execute optimistic block stream` to rollup node")
}
