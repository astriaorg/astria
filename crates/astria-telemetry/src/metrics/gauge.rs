use super::IntoF64;

/// A gauge.
#[derive(Clone)]
pub struct Gauge(metrics::Gauge);

impl Gauge {
    /// Increments the gauge.
    pub fn increment<T: IntoF64>(&self, value: T) {
        self.0.increment(value.into_f64());
    }

    /// Decrements the gauge.
    pub fn decrement<T: IntoF64>(&self, value: T) {
        self.0.decrement(value.into_f64());
    }

    /// Sets the gauge.
    pub fn set<T: IntoF64>(&self, value: T) {
        self.0.set(value.into_f64());
    }

    /// Creates a no-op gauge that does nothing.
    #[must_use]
    pub fn noop() -> Self {
        Self(metrics::Gauge::noop())
    }

    pub(super) fn new(gauge: metrics::Gauge) -> Self {
        Self(gauge)
    }
}
