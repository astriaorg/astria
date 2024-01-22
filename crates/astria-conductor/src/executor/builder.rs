use std::collections::HashMap;

use astria_core::{
    generated::execution::v1alpha2::execution_service_client::ExecutionServiceClient,
    sequencer::v1alpha1::RollupId,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::sync::{
    mpsc,
    oneshot,
    watch,
};
use tracing::info;

use super::Executor;
use crate::executor::{
    client,
    optimism,
};

pub(crate) struct NoRollupAddress;
pub(crate) struct WithRollupAddress(String);
pub(crate) struct NoRollupId;
pub(crate) struct WithRollupId(RollupId);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

pub(crate) struct ExecutorBuilder<
    TRollupAddress = NoRollupAddress,
    TRollupId = NoRollupId,
    TShutdown = NoShutdown,
> {
    optimism_hook: Option<optimism::Handler>,
    rollup_address: TRollupAddress,
    rollup_id: TRollupId,
    sequencer_height_with_first_rollup_block: u32,
    shutdown: TShutdown,
}

impl ExecutorBuilder {
    pub(super) fn new() -> Self {
        Self {
            optimism_hook: None,
            rollup_address: NoRollupAddress,
            rollup_id: NoRollupId,
            sequencer_height_with_first_rollup_block: 0,
            shutdown: NoShutdown,
        }
    }
}

impl ExecutorBuilder<WithRollupAddress, WithRollupId, WithShutdown> {
    pub(crate) async fn build(self) -> eyre::Result<Executor> {
        let Self {
            rollup_id,
            optimism_hook: pre_execution_hook,
            rollup_address,
            sequencer_height_with_first_rollup_block,
            shutdown,
        } = self;
        let WithRollupAddress(rollup_address) = rollup_address;
        let WithRollupId(rollup_id) = rollup_id;
        let WithShutdown(shutdown) = shutdown;

        let mut client = client::Client::from_execution_service_client(
            ExecutionServiceClient::connect(rollup_address)
                .await
                .wrap_err("failed to create execution rpc client")?,
        );
        let commitment_state = client
            .get_commitment_state()
            .await
            .wrap_err("to get initial commitment state")?;

        info!(
            soft_block_hash = %telemetry::display::hex(&commitment_state.soft().hash()),
            firm_block_hash = %telemetry::display::hex(&commitment_state.firm().hash()),
            "initial execution commitment state",
        );

        let (celestia_tx, celestia_rx) = mpsc::unbounded_channel();
        let (sequencer_tx, sequencer_rx) = mpsc::unbounded_channel();

        let state = watch::channel(super::State::new(
            commitment_state,
            sequencer_height_with_first_rollup_block,
        ))
        .0;

        Ok(Executor {
            celestia_rx,
            celestia_tx: celestia_tx.downgrade(),
            sequencer_rx,
            sequencer_tx: sequencer_tx.downgrade(),

            shutdown,
            client,
            rollup_id,
            state,
            blocks_pending_finalization: HashMap::new(),
            pre_execution_hook,
        })
    }
}

impl<TRollupAddress, TRollupId, TShutdown> ExecutorBuilder<TRollupAddress, TRollupId, TShutdown> {
    pub(crate) fn rollup_id(
        self,
        rollup_id: RollupId,
    ) -> ExecutorBuilder<TRollupAddress, WithRollupId, TShutdown> {
        let Self {
            optimism_hook,
            rollup_address,
            sequencer_height_with_first_rollup_block,
            shutdown,
            ..
        } = self;
        ExecutorBuilder {
            optimism_hook,
            rollup_address,
            rollup_id: WithRollupId(rollup_id),
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
    ) -> ExecutorBuilder<WithRollupAddress, TRollupId, TShutdown> {
        let Self {
            rollup_id,
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            shutdown,
            ..
        } = self;
        ExecutorBuilder {
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
    ) -> ExecutorBuilder<TRollupAddress, TRollupId, WithShutdown> {
        let Self {
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            rollup_address,
            rollup_id,
            ..
        } = self;
        ExecutorBuilder {
            optimism_hook,
            sequencer_height_with_first_rollup_block,
            rollup_address,
            rollup_id,
            shutdown: WithShutdown(shutdown),
        }
    }
}
