//! Initialize telemetry in all astria services.
//!
//! # Examples
//! ```
//! let metrics_conf = astria_telemetry::MetricsConfig {
//!     addr: "127.0.0.1:9000".parse().unwrap(),
//!     labels: vec![("label", "value")],
//!     buckets: None,
//! };
//! if let Err(err) = astria_telemetry::init(std::io::stdout, "info", Some(metrics_conf)) {
//!     eprintln!("failed to initialize telemetry: {err:?}");
//!     std::process::exit(1);
//! }
//! tracing::info!("telemetry initialized");
//! ```
use std::net::SocketAddr;

use metrics_exporter_prometheus::PrometheusBuilder;
use tracing_subscriber::{
    filter::ParseError,
    fmt::MakeWriter,
    layer::SubscriberExt as _,
    util::TryInitError,
};

pub mod display;

/// The errors that can occur when initializing telemtry.
#[derive(Debug)]
pub enum Error {
    FilterDirectives(ParseError),
    SubscriberInit(TryInitError),
}

impl From<TryInitError> for Error {
    fn from(err: TryInitError) -> Self {
        Self::SubscriberInit(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Self::FilterDirectives(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Error::FilterDirectives(_) => "could not parse provided filter directives",
            Error::SubscriberInit(_) => "could not install global tracing subscriber",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FilterDirectives(e) => Some(e),
            Self::SubscriberInit(e) => Some(e),
        }
    }
}

/// Configuration used for prometheus metrics.
pub struct MetricsConfig {
    /// Address to serve prometheus metrics on
    ///
    /// The HTTP listener that is spawned will respond to GET requests on any request path.
    pub addr: SocketAddr,
    /// List of labels to use as default globals for prometheus metrics.
    ///
    /// Labels specified on individual metrics will override these.
    pub labels: Vec<(&'static str, &'static str)>,
    /// Optionally sets the buckets to use when rendering histograms.
    ///
    /// If None, histograms will be rendered as summaries.
    ///
    /// Buckets values represent the higher bound of each buckets. If buckets are set, then all
    /// histograms will be rendered as true Prometheus histograms, instead of summaries.
    pub buckets: Option<Vec<f64>>,
}

/// Initializes telemtry, registering a global tracing subscriber, and metrics exporter.
///
/// This function installs a global [`tracing_subscriber::Registry`] to
/// record tracing spans and events. It detects if `stdout` of the executing
/// binary is a tty. If it is, a human readable output will be written to `sink`.
/// If `stdout` is not a tty, then json will be written to `sink.`
///
/// If `metrics_addr` is provided, a global prometheus metrics exporter will be
/// generated.
///
/// `sink` can be functions like `std::io::sink` or `std::io::stdout`.
/// `filter_directives` has to be a string like
/// `my_crate::module=debug,my_dependency=error`.
/// This will emit events in `my_crate::module` at debug level or higher, but
/// only error events in the entire `my_dependency` crate.
/// See [`tracing_subscriber::filter::EnvFilter::add_directive`] for more
/// information.
///
/// # Errors
///
/// Returns an error if `filter_directives` could not be parsed, or if the
/// global registry could not be installed.
///
/// # Panics
///
/// If a metrics url is provided and the prometheus metrics exporter fails to
/// install, this function will panic.
///
/// # Examples
///
/// Start telemetry with a global log level of `debug` writing to stdout.
/// ```
/// use tracing::{
///     debug,
///     info,
/// };
/// astria_telemetry::init(std::io::stdout, "info", None).unwrap();
/// info!("info events will be recorded");
/// debug!("but debug events will not");
/// ```
///
/// Don't write any events by sending them to `std::io::sink`. This is mainly
/// useful in tests because `tracing` circumvents rust's mechanism to capture
/// stdout/stderr.
/// ```
/// use tracing::info;
/// astria_telemetry::init(std::io::sink, "info", None).unwrap();
/// info!("this will not be logged because of `std::io::sink`");
/// ```
pub fn init<S>(
    sink: S,
    filter_directives: &str,
    metrics_conf: Option<MetricsConfig>,
) -> Result<(), Error>
where
    S: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    init_logging(sink, filter_directives)?;

    if let Some(metrics_conf) = metrics_conf {
        init_metrics(metrics_conf);
    }

    Ok(())
}

fn init_metrics(conf: MetricsConfig) {
    let mut metrics_builder = PrometheusBuilder::new();

    for (key, value) in conf.labels {
        metrics_builder = metrics_builder.add_global_label(key, value);
    }

    if let Some(buckets) = conf.buckets {
        metrics_builder = metrics_builder
            .set_buckets(&buckets)
            .expect("failed to set prometheus buckets");
    }

    metrics_builder
        .with_http_listener(conf.addr)
        .install()
        .expect("failed to install prometheus metrics exporter")
}

fn init_logging<S>(sink: S, filter_directives: &str) -> Result<(), Error>
where
    S: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    use std::io::IsTerminal as _;

    use tracing_subscriber::{
        filter::{
            EnvFilter,
            LevelFilter,
        },
        fmt,
        registry,
        util::SubscriberInitExt as _,
    };
    let env_filter = {
        let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
        builder.parse(filter_directives)?
    };
    let (json_log, stdout_log) = if std::io::stdout().is_terminal() {
        eprintln!("service is attached to tty; using human readable formatting");
        (None, Some(fmt::layer().with_writer(sink)))
    } else {
        eprintln!("service is not attached to tty; using json formatting");
        (
            Some(fmt::layer().json().flatten_event(true).with_writer(sink)),
            None,
        )
    };

    Ok(registry()
        .with(stdout_log)
        .with(json_log)
        .with(env_filter)
        .try_init()?)
}
