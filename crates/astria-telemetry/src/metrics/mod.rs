mod builders;
mod counter;
mod error;
mod factories;
mod gauge;
mod handle;
mod histogram;
mod into_f64;

use metrics_exporter_prometheus::PrometheusBuilder;

pub use self::{
    builders::{
        BucketBuilder,
        ConfigBuilder,
        RegisteringBuilder,
    },
    counter::Counter,
    error::Error,
    factories::{
        CounterFactory,
        GaugeFactory,
        HistogramFactory,
    },
    gauge::Gauge,
    handle::Handle,
    histogram::Histogram,
    into_f64::IntoF64,
};

pub trait Metrics {
    type Config;

    /// Sets the histograms' buckets as required.
    ///
    /// If not set for a given histogram, it will be rendered as a Prometheus summary rather than a
    /// histogram.
    ///
    /// # Errors
    ///
    /// Implementations should return an error if setting buckets fails.
    fn set_buckets(_builder: &mut BucketBuilder, _config: &Self::Config) -> Result<(), Error> {
        Ok(())
    }

    /// Registers the individual metrics as required and returns an instance of `Self`.
    ///
    /// # Errors
    ///
    /// Implementations should return an error if registering metrics fails.
    fn register(builder: &mut RegisteringBuilder, config: &Self::Config) -> Result<Self, Error>
    where
        Self: Sized;

    /// Returns an instance of `Self` where the metrics are registered to a recorder that is
    /// dropped immediately, meaning metrics aren't recorded.
    ///
    /// # Errors
    ///
    /// Implementations should return an error if setting buckets fails.
    fn noop_metrics(config: &Self::Config) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut builder = RegisteringBuilder::new(PrometheusBuilder::new().build_recorder());
        Self::register(&mut builder, config)
    }
}
