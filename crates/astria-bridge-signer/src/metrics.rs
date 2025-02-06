use telemetry::{
    metric_names,
    metrics::{
        self,
        Counter,
        Gauge,
        Histogram,
        RegisteringBuilder,
    },
};

pub struct Metrics {}

impl metrics::Metrics for Metrics {
    type Config = ();

    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        todo!()
    }
}
