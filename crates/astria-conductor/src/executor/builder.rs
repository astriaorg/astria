use std::collections::HashMap;

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{
    state,
    Executor,
    Handle,
    StateNotInit,
};
use crate::config::CommitLevel;

pub(crate) struct Builder {
    pub(crate) mode: CommitLevel,
    pub(crate) rollup_address: String,
    pub(crate) shutdown: CancellationToken,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(Executor, Handle)> {
        let Self {
            mode,
            rollup_address,
            shutdown,
        } = self;

        let rollup_address = rollup_address
            .parse()
            .wrap_err("failed to parse rollup address as URI")?;

        let mut firm_block_tx = None;
        let mut firm_block_rx = None;
        if mode.is_with_firm() {
            let (tx, rx) = mpsc::channel(16);
            firm_block_tx = Some(tx);
            firm_block_rx = Some(rx);
        }

        let mut soft_block_tx = None;
        let mut soft_block_rx = None;
        if mode.is_with_soft() {
            let (tx, rx) = super::soft_block_channel();
            soft_block_tx = Some(tx);
            soft_block_rx = Some(rx);
        }

        let (state_tx, state_rx) = state::channel();

        let executor = Executor {
            mode,

            firm_blocks: firm_block_rx,
            soft_blocks: soft_block_rx,

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
