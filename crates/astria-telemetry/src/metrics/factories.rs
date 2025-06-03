use std::{
    collections::BTreeSet,
    marker::PhantomData,
    sync::Arc,
};

use metrics::{
    Key,
    Label,
    Metadata,
    Recorder,
};

use super::{
    Counter,
    Error,
    Gauge,
    Histogram,
};

pub struct CounterFactory(Factory<Counter>);

impl CounterFactory {
    /// Registers and returns a counter with no labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with no labels.
    pub fn register(&mut self) -> Result<Counter, Error> {
        self.0.register()
    }

    /// Registers and returns a counter with the given labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with the same labels (regardless
    /// of order of the labels) or if any of the label pairs are duplicates.
    pub fn register_with_labels(
        &mut self,
        labels: &[(&'static str, String)],
    ) -> Result<Counter, Error> {
        self.0.register_with_labels(labels)
    }

    pub(super) fn new(name: &'static str, recorder: Arc<dyn Recorder>) -> Self {
        Self(Factory::new(name, recorder))
    }

    pub(super) fn metric_type() -> &'static str {
        Factory::<Counter>::metric_type()
    }
}

pub struct GaugeFactory(Factory<Gauge>);

impl GaugeFactory {
    /// Registers and returns a gauge with no labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with no labels.
    pub fn register(&mut self) -> Result<Gauge, Error> {
        self.0.register()
    }

    /// Registers and returns a gauge with the given labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with the same labels (regardless
    /// of order of the labels) or if any of the label pairs are duplicates.
    pub fn register_with_labels(
        &mut self,
        labels: &[(&'static str, String)],
    ) -> Result<Gauge, Error> {
        self.0.register_with_labels(labels)
    }

    pub(super) fn new(name: &'static str, recorder: Arc<dyn Recorder>) -> Self {
        Self(Factory::new(name, recorder))
    }

    pub(super) fn metric_type() -> &'static str {
        Factory::<Gauge>::metric_type()
    }
}

pub struct HistogramFactory(Factory<Histogram>);

impl HistogramFactory {
    /// Registers and returns a histogram with no labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with no labels.
    pub fn register(&mut self) -> Result<Histogram, Error> {
        self.0.register()
    }

    /// Registers and returns a histogram with the given labels applied.
    ///
    /// # Errors
    ///
    /// Returns an error if this metric has already been registered with the same labels (regardless
    /// of order of the labels) or if any of the label pairs are duplicates.
    pub fn register_with_labels(
        &mut self,
        labels: &[(&'static str, String)],
    ) -> Result<Histogram, Error> {
        self.0.register_with_labels(labels)
    }

    pub(super) fn new(name: &'static str, recorder: Arc<dyn Recorder>) -> Self {
        Self(Factory::new(name, recorder))
    }

    pub(super) fn metric_type() -> &'static str {
        Factory::<Histogram>::metric_type()
    }
}

struct Factory<T> {
    name: &'static str,
    recorder: Arc<dyn Recorder>,
    labels: BTreeSet<BTreeSet<Label>>,
    _phantom: PhantomData<T>,
}

impl<T> Factory<T>
where
    Factory<T>: RegisterMetric<T>,
{
    fn register(&mut self) -> Result<T, Error> {
        self.register_with_labels(&[])
    }

    fn register_with_labels(&mut self, labels: &[(&'static str, String)]) -> Result<T, Error> {
        let key = Key::from_parts(self.name, labels);

        let mut unique_labels = BTreeSet::new();
        for label in key.labels() {
            if !unique_labels.insert(label.clone()) {
                return Err(Error::DuplicateLabel {
                    metric_type: Self::metric_type(),
                    metric_name: self.name,
                    label_name: label.key().to_string(),
                    label_value: label.value().to_string(),
                });
            }
        }

        if !self.labels.insert(unique_labels) {
            return Err(Error::MetricAlreadyRegistered {
                metric_type: Self::metric_type(),
                metric_name: self.name,
            });
        }

        Ok(self.register_metric(&key))
    }

    fn new(name: &'static str, recorder: Arc<dyn Recorder>) -> Self {
        Self {
            name,
            recorder,
            labels: BTreeSet::new(),
            _phantom: PhantomData,
        }
    }
}

trait RegisterMetric<T> {
    fn register_metric(&self, key: &Key) -> T;

    fn metric_type() -> &'static str;
}

impl RegisterMetric<Counter> for Factory<Counter> {
    fn register_metric(&self, key: &Key) -> Counter {
        let ignored_metadata = Metadata::new("", metrics::Level::ERROR, None);
        Counter::new(self.recorder.register_counter(key, &ignored_metadata))
    }

    fn metric_type() -> &'static str {
        "counter"
    }
}

impl RegisterMetric<Gauge> for Factory<Gauge> {
    fn register_metric(&self, key: &Key) -> Gauge {
        let ignored_metadata = Metadata::new("", metrics::Level::ERROR, None);
        Gauge::new(self.recorder.register_gauge(key, &ignored_metadata))
    }

    fn metric_type() -> &'static str {
        "gauge"
    }
}

impl RegisterMetric<Histogram> for Factory<Histogram> {
    fn register_metric(&self, key: &Key) -> Histogram {
        let ignored_metadata = Metadata::new("", metrics::Level::ERROR, None);
        Histogram::new(self.recorder.register_histogram(key, &ignored_metadata))
    }

    fn metric_type() -> &'static str {
        "histogram"
    }
}
