use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio_util::sync::CancellationToken;

use super::{
    running::Running,
    starting::Starting,
};
use crate::{
    Config,
    Metrics,
};

/// The implementation of the auctioneer business logic.
pub(super) struct Inner {
    run_state: RunState,
}

impl Inner {
    /// Creates an [`Auctioneer`] service from a [`Config`] and [`Metrics`].
    pub(super) fn new(
        cfg: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let run_state = super::starting::run_state(cfg, shutdown_token, metrics)
            .wrap_err("failed initializating in starting state")?;
        Ok(Self {
            run_state,
        })
    }

    /// Runs the [`Auctioneer`] service until it received an exit signal, or one of the constituent
    /// tasks either ends unexpectedly or returns an error.
    pub(super) async fn run(self) -> eyre::Result<()> {
        let Self {
            mut run_state,
        } = self;

        loop {
            match run_state {
                RunState::Cancelled => break Ok(()),
                RunState::Starting(starting) => match starting.run().await {
                    Ok(new_state) => run_state = new_state,
                    Err(err) => break Err(err).wrap_err("failed during startup"),
                },
                RunState::Running(running) => match running.run().await {
                    Ok(new_state) => run_state = new_state,
                    Err(err) => break Err(err).wrap_err("failed during execution"),
                },
            }
        }
    }
}

pub(super) enum RunState {
    Cancelled,
    Starting(Starting),
    Running(Running),
}

impl From<Running> for RunState {
    fn from(value: Running) -> Self {
        Self::Running(value)
    }
}

impl From<Starting> for RunState {
    fn from(value: Starting) -> Self {
        Self::Starting(value)
    }
}
