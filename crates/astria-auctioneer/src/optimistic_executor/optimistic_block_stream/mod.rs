mod builder;
pub(crate) use builder::Builder;

use crate::Metrics;

pub(crate) struct OptimisticBlockStream {
    #[allow(dead_code)]
    metrics: &'static Metrics,
}
