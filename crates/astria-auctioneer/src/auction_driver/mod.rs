use astria_eyre::eyre;

use crate::Metrics;

mod builder;
pub(crate) use builder::Builder;

pub(crate) struct AuctionDriver {
    #[allow(dead_code)]
    metrics: &'static Metrics,
}

impl AuctionDriver {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        todo!("implement me")
    }
}
