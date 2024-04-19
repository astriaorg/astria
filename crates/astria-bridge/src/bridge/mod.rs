use std::sync::Arc;

use astria_eyre::eyre;
use tokio_util::sync::CancellationToken;

mod builder;
mod state;

pub(crate) use builder::Builder;
use state::State;
pub(crate) use state::StateSnapshot;

pub struct Bridge {
    shutdown_token: CancellationToken,
    state: Arc<State>,
}

impl Bridge {
    pub(crate) fn subscribe_to_state(&self) -> tokio::sync::watch::Receiver<StateSnapshot> {
        self.state.subscribe()
    }

    pub(crate) async fn run(self) -> eyre::Result<()> {
        Ok(())
    }
}
