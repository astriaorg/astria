use std::collections::HashMap;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::sync::{
    mpsc,
    oneshot,
    watch,
};

use super::Executor;
use crate::executor::optimism;

pub(crate) struct NoRollupAddress;
pub(crate) struct WithRollupAddress(tonic::transport::Uri);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

pub(crate) struct ExecutorBuilder<TRollupAddress = NoRollupAddress, TShutdown = NoShutdown> {
    optimism_hook: Option<optimism::Handler>,
    rollup_address: TRollupAddress,
    shutdown: TShutdown,
}

impl ExecutorBuilder {
    pub(super) fn new() -> Self {
        Self {
            optimism_hook: None,
            rollup_address: NoRollupAddress,
            shutdown: NoShutdown,
        }
    }
}

impl ExecutorBuilder<WithRollupAddress, WithShutdown> {
    pub(crate) fn build(self) -> Executor {
        let Self {
            optimism_hook: pre_execution_hook,
            rollup_address,
            shutdown,
        } = self;
        let WithRollupAddress(rollup_address) = rollup_address;
        let WithShutdown(shutdown) = shutdown;

        let (celestia_tx, celestia_rx) = mpsc::unbounded_channel();
        let (sequencer_tx, sequencer_rx) = mpsc::unbounded_channel();

        let state = watch::channel(super::State::new()).0;

        Executor {
            celestia_rx,
            celestia_tx: celestia_tx.downgrade(),
            sequencer_rx,
            sequencer_tx: sequencer_tx.downgrade(),

            rollup_address,

            shutdown,
            state,
            blocks_pending_finalization: HashMap::new(),
            pre_execution_hook,
        }
    }
}

impl<TRollupAddress, TShutdown> ExecutorBuilder<TRollupAddress, TShutdown> {
    pub(crate) fn set_optimism_hook(mut self, handler: Option<optimism::Handler>) -> Self {
        self.optimism_hook = handler;
        self
    }

    pub(crate) fn rollup_address(
        self,
        rollup_address: &str,
    ) -> eyre::Result<ExecutorBuilder<WithRollupAddress, TShutdown>> {
        let Self {
            optimism_hook,
            shutdown,
            ..
        } = self;
        let rollup_address = WithRollupAddress(
            rollup_address
                .parse()
                .wrap_err("failed to parse rollup address as URI")?,
        );
        Ok(ExecutorBuilder {
            optimism_hook,
            rollup_address,
            shutdown,
        })
    }

    pub(crate) fn shutdown(
        self,
        shutdown: oneshot::Receiver<()>,
    ) -> ExecutorBuilder<TRollupAddress, WithShutdown> {
        let Self {
            optimism_hook,
            rollup_address,
            ..
        } = self;
        ExecutorBuilder {
            optimism_hook,
            rollup_address,
            shutdown: WithShutdown(shutdown),
        }
    }
}
