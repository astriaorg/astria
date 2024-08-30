/// A counter.
#[derive(Clone)]
pub struct Counter(metrics::Counter);

impl Counter {
    /// Increments the counter.
    pub fn increment(&self, value: u64) {
        self.0.increment(value);
    }

    /// Sets the counter to an absolute value.
    pub fn absolute(&self, value: u64) {
        self.0.absolute(value);
    }

    /// Creates a no-op counter that does nothing.
    #[must_use]
    pub fn noop() -> Self {
        Self(metrics::Counter::noop())
    }

    pub(super) fn new(counter: metrics::Counter) -> Self {
        Self(counter)
    }
}
