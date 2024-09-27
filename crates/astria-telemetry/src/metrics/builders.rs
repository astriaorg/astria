use std::{
    collections::{
        hash_map::Entry,
        HashMap,
        HashSet,
    },
    mem,
    net::SocketAddr,
};

use metrics::Recorder as _;
use metrics_exporter_prometheus::{
    ExporterFuture,
    Matcher,
    PrometheusBuilder,
    PrometheusRecorder,
};

#[cfg(doc)]
use super::{
    Counter,
    Gauge,
    Histogram,
};
use super::{
    CounterFactory,
    Error,
    GaugeFactory,
    Handle,
    HistogramFactory,
    Metrics,
};

/// A builder used to gather metrics settings, register metrics, start the exporter server and
/// register the global metrics recorder.
pub struct ConfigBuilder {
    service_name: String,
    listening_address: Option<String>,
    use_global_recorder: bool,
}

impl ConfigBuilder {
    /// Returns a new `ConfigBuilder`.
    ///
    /// If [`Self::with_listener_address`] is not called, no http server will be started, meaning
    /// the metrics' values can only be rendered via the [`Handle`] returned by [`Self::build`].
    ///
    /// By default, a global metrics recorder will be set when calling `Self::build`. This can be
    /// disabled by calling [`Self::with_global_recorder(false)`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            service_name: String::new(),
            listening_address: None,
            use_global_recorder: true,
        }
    }

    /// All metrics will have a label applied of `service="<service_name>"`.
    ///
    /// If `service_name` is empty, the label is not applied.
    #[must_use]
    pub fn set_service_name(mut self, service_name: &str) -> Self {
        self.service_name = service_name.to_string();
        self
    }

    /// Sets the listening address of the exporter server.
    #[must_use]
    pub fn set_listening_address(mut self, listening_address: &str) -> Self {
        self.listening_address = Some(listening_address.to_string());
        self
    }

    /// Enables or disables setting the global metrics recorder.
    #[must_use]
    pub fn set_global_recorder(mut self, use_global_recorder: bool) -> Self {
        self.use_global_recorder = use_global_recorder;
        self
    }

    /// Registers the buckets and metrics as specified in `T::set_buckets` and `T::register`
    /// respectively, starts the http server if enabled, sets the global metrics recorder if
    /// requested and returns a new metrics object of type `T` along with a handle for rendering
    /// current metrics.
    // allow: no useful error info can be added without writing excessive details.
    #[allow(clippy::missing_errors_doc)]
    pub fn build<T: Metrics>(self, config: &T::Config) -> Result<(T, Handle), Error> {
        // Apply settings to the prometheus builder.
        let mut prometheus_builder = PrometheusBuilder::new();
        if !self.service_name.is_empty() {
            prometheus_builder = prometheus_builder.add_global_label("service", self.service_name);
        }
        if let Some(listening_address) = &self.listening_address {
            let addr: SocketAddr = listening_address.parse()?;
            prometheus_builder = prometheus_builder.with_http_listener(addr);
        }

        // Set the histogram buckets.
        let mut bucket_builder = BucketBuilder {
            builder: prometheus_builder,
            buckets: HashMap::new(),
        };
        T::set_buckets(&mut bucket_builder, config)?;
        let histograms_with_buckets = bucket_builder.histogram_names();

        // Consume the prometheus builder, yielding a recorder and a future for running the exporter
        // server (this will be a no-op if the server isn't configured to run).
        let (recorder, exporter_fut) = if self.listening_address.is_some() {
            bucket_builder
                .builder
                .build()
                .map_err(|error| Error::StartListening(error.into()))?
        } else {
            let recorder = bucket_builder.builder.build_recorder();
            let fut: ExporterFuture = Box::pin(async move { Ok(()) });
            (recorder, fut)
        };
        let handle = Handle::new(recorder.handle());

        // Register individual metrics.
        let mut registering_builder = RegisteringBuilder::new(recorder);
        let metrics = T::register(&mut registering_builder, config)?;

        // Ensure no histogram buckets were left unassigned.
        let unassigned: HashSet<_> = histograms_with_buckets
            .difference(&registering_builder.histograms)
            .cloned()
            .collect();
        if !unassigned.is_empty() {
            return Err(Error::BucketsNotAssigned(unassigned));
        }

        // Run the exporter server and set the global recorder if requested.
        tokio::spawn(exporter_fut);
        if self.use_global_recorder {
            metrics::set_global_recorder(registering_builder.recorder)
                .map_err(|_| Error::GlobalMetricsRecorderAlreadySet)?;
        }
        Ok((metrics, handle))
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder used to set histogram buckets.
///
/// It is constructed in [`ConfigBuilder::build`] and passed to [`Metrics::set_buckets`].
pub struct BucketBuilder {
    builder: PrometheusBuilder,
    buckets: HashMap<&'static str, Vec<f64>>,
}

impl BucketBuilder {
    /// Sets the buckets for the given histogram.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` is empty, or if `histogram_name` has already had buckets set.
    ///
    /// If the given histogram is not registered later via the `RegisteringBuilder`, then
    /// `RegisteringBuilder::build` will return an error.
    pub fn set_buckets(
        &mut self,
        histogram_name: &'static str,
        values: &[f64],
    ) -> Result<(), Error> {
        match self.buckets.entry(histogram_name) {
            Entry::Occupied(_) => return Err(Error::BucketsAlreadySet(histogram_name)),
            Entry::Vacant(entry) => {
                let _ = entry.insert(values.to_vec());
            }
        }
        // Swap out the builder temporarily to call `set_buckets_for_metric` which consumes the
        // builder, then swap it back into `self.builder`.
        let mut builder = mem::take(&mut self.builder)
            .set_buckets_for_metric(Matcher::Full(histogram_name.to_string()), values)
            .map_err(|_| Error::EmptyBuckets(histogram_name))?;
        mem::swap(&mut builder, &mut self.builder);
        Ok(())
    }

    fn histogram_names(&self) -> HashSet<String> {
        self.buckets.keys().map(ToString::to_string).collect()
    }
}

/// A builder used to register individual metrics.
///
/// It is constructed in [`ConfigBuilder::build`] and passed to [`Metrics::register`].
pub struct RegisteringBuilder {
    recorder: PrometheusRecorder,
    counters: HashSet<String>,
    gauges: HashSet<String>,
    histograms: HashSet<String>,
}

impl RegisteringBuilder {
    /// Returns a new `CounterFactory` for registering [`Counter`]s under the given name.
    ///
    /// # Errors
    ///
    /// Returns an error if a counter has already been registered under this name.
    pub fn new_counter_factory(
        &mut self,
        name: &'static str,
        description: &'static str,
    ) -> Result<CounterFactory, Error> {
        if !self.counters.insert(name.to_string()) {
            return Err(Error::MetricAlreadyRegistered {
                metric_type: CounterFactory::metric_type(),
                metric_name: name,
            });
        }

        self.recorder
            .describe_counter(name.into(), None, description.into());
        Ok(CounterFactory::new(name, &self.recorder))
    }

    /// Returns a new `GaugeFactory` for registering [`Gauge`]s under the given name.
    ///
    /// # Errors
    ///
    /// Returns an error if a gauge has already been registered under this name.
    pub fn new_gauge_factory(
        &mut self,
        name: &'static str,
        description: &'static str,
    ) -> Result<GaugeFactory, Error> {
        if !self.gauges.insert(name.to_string()) {
            return Err(Error::MetricAlreadyRegistered {
                metric_type: GaugeFactory::metric_type(),
                metric_name: name,
            });
        }

        self.recorder
            .describe_gauge(name.into(), None, description.into());
        Ok(GaugeFactory::new(name, &self.recorder))
    }

    /// Returns a new `HistogramFactory` for registering [`Histogram`]s under the given name.
    ///
    /// # Errors
    ///
    /// Returns an error if a histogram has already been registered under this name.
    pub fn new_histogram_factory(
        &mut self,
        name: &'static str,
        description: &'static str,
    ) -> Result<HistogramFactory, Error> {
        if !self.histograms.insert(name.to_string()) {
            return Err(Error::MetricAlreadyRegistered {
                metric_type: HistogramFactory::metric_type(),
                metric_name: name,
            });
        }

        self.recorder
            .describe_histogram(name.into(), None, description.into());
        Ok(HistogramFactory::new(name, &self.recorder))
    }

    pub(super) fn new(recorder: PrometheusRecorder) -> Self {
        RegisteringBuilder {
            recorder,
            counters: HashSet::new(),
            gauges: HashSet::new(),
            histograms: HashSet::new(),
        }
    }
}
