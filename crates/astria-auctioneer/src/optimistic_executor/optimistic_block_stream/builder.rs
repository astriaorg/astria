use astria_eyre::eyre;

use super::OptimisticBlockStream;
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<OptimisticBlockStream> {
        let Self {
            metrics,
        } = self;

        Ok(OptimisticBlockStream {
            metrics,
        })
    }
}
