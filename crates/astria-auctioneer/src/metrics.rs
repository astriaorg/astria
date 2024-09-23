use telemetry::{
    metric_names,
    metrics::{
        self,
        RegisteringBuilder,
    },
};

pub struct Metrics {}

impl Metrics {}

impl metrics::Metrics for Metrics {
    type Config = ();

    fn register(
        _builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        Ok(Self {})
    }
}

metric_names!(const METRICS_NAMES:
);
