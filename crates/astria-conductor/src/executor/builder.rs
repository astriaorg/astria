use std::collections::HashMap;

use astria_eyre::eyre::{
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

pub(crate) struct NoRollupAddress;
pub(crate) struct WithRollupAddress(tonic::transport::Uri);
pub(crate) struct NoShutdown;
pub(crate) struct WithShutdown(oneshot::Receiver<()>);

pub(crate) struct ExecutorBuilder<TRollupAddress = NoRollupAddress, TShutdown = NoShutdown> {
    consider_commitment_spread: bool,
    rollup_address: TRollupAddress,
    shutdown: TShutdown,
}

impl ExecutorBuilder {
    pub(super) fn new() -> Self {
        Self {
            consider_commitment_spread: true,
            rollup_address: NoRollupAddress,
            shutdown: NoShutdown,
        }
    }
}

impl ExecutorBuilder<WithRollupAddress, WithShutdown> {
    pub(crate) fn build(self) -> (Executor, Handle) {
        let Self {
            consider_commitment_spread,
            rollup_address,
            shutdown,
        } = self;
        let WithRollupAddress(rollup_address) = rollup_address;
        let WithShutdown(shutdown) = shutdown;

        let (firm_block_tx, firm_block_rx) = mpsc::channel(16);
        let (soft_block_tx, soft_block_rx) = super::soft_block_channel();

        let (state_tx, state_rx) = watch::channel(State::new());

        let executor = Executor {
            firm_blocks: firm_block_rx,
            soft_blocks: soft_block_rx,

            consider_commitment_spread,
            rollup_address,

            shutdown,
            state: state_tx,
            blocks_pending_finalization: HashMap::new(),
        };
        let handle = Handle {
            firm_blocks: firm_block_tx,
            soft_blocks: soft_block_tx,
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

    pub(crate) fn rollup_address(
        self,
        rollup_address: &str,
    ) -> eyre::Result<ExecutorBuilder<WithRollupAddress, TShutdown>> {
        let Self {
            consider_commitment_spread,
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
            rollup_address,
            ..
        } = self;
        ExecutorBuilder {
            consider_commitment_spread,
            rollup_address,
            shutdown: WithShutdown(shutdown),
        }
    }
}
