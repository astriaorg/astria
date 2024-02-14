use std::collections::HashMap;

use eyre::{
    self,
    WrapErr as _,
};
use tokio::sync::{
    mpsc,
    oneshot,
    watch,
};

use super::{
    Executor,
    Handle,
    State,
    StateNotInit,
};
use crate::executor::optimism;

pub(crate) struct NoRollupAddress;
pub(crate) struct WithRollupAddress(tonic::transport::Uri);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

pub(crate) struct ExecutorBuilder<TRollupAddress = NoRollupAddress, TShutdown = NoShutdown> {
    consider_commitment_spread: bool,
    optimism_hook: Option<optimism::Handler>,
    rollup_address: TRollupAddress,
    shutdown: TShutdown,
}

impl ExecutorBuilder {
    pub(super) fn new() -> Self {
        Self {
            consider_commitment_spread: true,
            optimism_hook: None,
            rollup_address: NoRollupAddress,
            shutdown: NoShutdown,
        }
    }
}

impl ExecutorBuilder<WithRollupAddress, WithShutdown> {
    pub(crate) fn build(self) -> (Executor, Handle) {
        let Self {
            consider_commitment_spread,
            optimism_hook: pre_execution_hook,
            rollup_address,
            shutdown,
        } = self;
        let WithRollupAddress(rollup_address) = rollup_address;
        let WithShutdown(shutdown) = shutdown;

        let (firm_blocks_tx, firm_blocks_rx) = mpsc::channel(16);
        let (soft_blocks_tx, soft_blocks_rx) = mpsc::channel(16);

        let (state_tx, state_rx) = watch::channel(State::new());

        let executor = Executor {
            firm_blocks: firm_blocks_rx,
            soft_blocks: soft_blocks_rx,

            consider_commitment_spread,
            rollup_address,

            shutdown,
            state: state_tx,
            blocks_pending_finalization: HashMap::new(),
            pre_execution_hook,
        };
        let handle = Handle {
            firm_blocks: firm_blocks_tx,
            soft_blocks: soft_blocks_tx,
            state: state_rx,
            _state_init: StateNotInit,
        };
        (executor, handle)
    }
}

impl<TRollupAddress, TShutdown> ExecutorBuilder<TRollupAddress, TShutdown> {
    pub(crate) fn set_consider_commitment_spread(
        mut self,
        consider_commitment_spread: bool,
    ) -> Self {
        self.consider_commitment_spread = consider_commitment_spread;
        self
    }

    pub(crate) fn set_optimism_hook(mut self, handler: Option<optimism::Handler>) -> Self {
        self.optimism_hook = handler;
        self
    }

    pub(crate) fn rollup_address(
        self,
        rollup_address: &str,
    ) -> eyre::Result<ExecutorBuilder<WithRollupAddress, TShutdown>> {
        let Self {
            consider_commitment_spread,
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
            consider_commitment_spread,
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
            consider_commitment_spread,
            optimism_hook,
            rollup_address,
            ..
        } = self;
        ExecutorBuilder {
            consider_commitment_spread,
            optimism_hook,
            rollup_address,
            shutdown: WithShutdown(shutdown),
        }
    }
}
