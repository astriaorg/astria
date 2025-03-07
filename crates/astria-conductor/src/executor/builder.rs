use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio_util::sync::CancellationToken;

use super::Executor;
use crate::metrics::Metrics;

pub(crate) struct Builder {
    pub(crate) config: crate::Config,
    pub(crate) shutdown: CancellationToken,
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<Executor> {
        let Self {
            config,
            shutdown,
            metrics,
        } = self;

        let client =
            super::client::Client::connect_lazy(&config.execution_rpc_url).wrap_err_with(|| {
                format!(
                    "failed to construct execution client for provided rollup address `{}`",
                    config.execution_rpc_url,
                )
            })?;

        let executor = Executor {
            config,
            client,
            shutdown,
            metrics,
        };
        Ok(executor)
    }
}
