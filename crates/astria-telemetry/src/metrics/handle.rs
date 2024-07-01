use metrics_exporter_prometheus::PrometheusHandle;

/// A handle for rendering a snapshot of current metrics.
#[derive(Clone)]
pub struct Handle(PrometheusHandle);

impl Handle {
    /// Renders the current metrics in the same form as that of the metrics http server.
    #[must_use]
    pub fn render(&self) -> String {
        self.0.render()
    }

    pub(super) fn new(handle: PrometheusHandle) -> Self {
        Self(handle)
    }
}
