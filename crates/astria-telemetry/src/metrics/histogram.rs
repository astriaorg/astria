use super::IntoF64;

/// A histogram.
#[derive(Clone)]
pub struct Histogram(metrics::Histogram);

impl Histogram {
    /// Records a value in the histogram.
    pub fn record<T: IntoF64>(&self, value: T) {
        self.0.record(value.into_f64());
    }

    /// Creates a no-op histogram that does nothing.
    #[must_use]
    pub fn noop() -> Self {
        Self(metrics::Histogram::noop())
    }

    pub(super) fn new(histogram: metrics::Histogram) -> Self {
        Self(histogram)
    }
}
