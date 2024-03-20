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

pub(crate) struct Builder {
    pub(crate) consider_commitment_spread: bool,
    pub(crate) rollup_address: String,
    pub(crate) shutdown: oneshot::Receiver<()>,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(Executor, Handle)> {
        let Self {
            consider_commitment_spread,
            rollup_address,
            shutdown,
        } = self;

        let rollup_address = rollup_address
            .parse()
            .wrap_err("failed to parse rollup address as URI")?;

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
        Ok((executor, handle))
    }
}
