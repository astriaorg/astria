use astria_eyre::eyre;

use super::AuctionDriver;
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<AuctionDriver> {
        let Self {
            metrics,
        } = self;

        Ok(AuctionDriver {
            metrics,
        })
    }
}
