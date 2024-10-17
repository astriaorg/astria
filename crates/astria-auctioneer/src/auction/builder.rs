use astria_eyre::eyre;

use super::driver::Driver;
use crate::Metrics;

pub(crate) struct Builder {
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<Driver> {
        unimplemented!()
    }
}
