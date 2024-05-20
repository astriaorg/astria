use std::sync::Arc;

use astria_eyre::eyre;
use tokio_util::sync::CancellationToken;

mod builder;
mod state;

pub(crate) use builder::Builder;
use state::State;
pub(super) use state::StateSnapshot;

pub(super) struct Bridge {
    _shutdown_token: CancellationToken,
    state: Arc<State>,
}

impl Bridge {
    pub(super) fn new(state: Arc<State>, shutdown_token: &CancellationToken) -> Self {
        Self {
            state,
            _shutdown_token: shutdown_token.clone(),
        }
    }

    pub(super) fn subscribe_to_state(&self) -> tokio::sync::watch::Receiver<StateSnapshot> {
        self.state.subscribe()
    }

    #[allow(clippy::unused_async)]
    pub(super) async fn run(self) -> eyre::Result<()> {
        self.state.set_ready();
        Ok(())
    }
}
