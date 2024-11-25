use std::time::Duration;

use astria_core::primitive::v1::RollupId;
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use futures::StreamExt as _;
use tokio_util::sync::CancellationToken;

use super::{
    inner::RunState,
    running::Running,
};
use crate::{
    rollup_channel::RollupChannel,
    sequencer_channel::SequencerChannel,
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
        let Self {
            auctions,
            rollup_id,
            rollup_channel,
            sequencer_channel,
            shutdown_token,
        } = self;

        let executed_blocks = rollup_channel
            .open_execute_optimistic_block_stream()
            .await
            .wrap_err("opening stream to execute optimistic blocks on rollup failed")?;

        let mut optimistic_blocks = sequencer_channel
            .open_get_optimistic_block_stream(rollup_id)
            .await
            .wrap_err("opening stream to receive optimistic blocks from sequencer failed")?;

        let block_commitments = sequencer_channel
            .open_get_block_commitment_stream()
            .await
            .wrap_err("opening stream to receive block commitments from sequencer failed")?;

        let bundles = rollup_channel
            .open_bundle_stream()
            .await
            .wrap_err("opening stream to receive bundles from rollup failed")?;

        let optimistic_block = optimistic_blocks
            .next()
            .await
            .ok_or_eyre("optimistic stream closed during startup?")?
            .wrap_err("failed to get optimistic block during startup")?;
        let current_block = crate::block::Current::with_optimistic(optimistic_block);

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
