use std::sync::Arc;

use astria_eyre::eyre;
use tokio_util::sync::CancellationToken;

use super::{
    state::State,
    Bridge,
};

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
}

impl Builder {
    /// Instantiates a `Bridge`.
    pub(crate) fn build(self) -> eyre::Result<super::Bridge> {
        let Self {
            shutdown_token,
        } = self;

        let state = Arc::new(State::new());

        Ok(Bridge::new(state, shutdown_token))
    }
}
