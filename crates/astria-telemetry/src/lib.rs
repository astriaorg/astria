//! Initialize telemetry in all astria services.
//!
//! # Examples
//! ```
//! if let Err(err) = astria_telemetry::init(std::io::stdout, "info") {
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

/// Register a global tracing subscriber.
///
/// This function installs a global [`tracing_subscriber::Registry`] to
/// record tracing spans and events. It detects if `stdout` of the executing
/// binary is a tty. If it is, a human readable output will be written to `sink`.
/// If `stdout` is not a tty, then json will be written to `sink.`
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
    metrics_addr: Option<SocketAddr>,
) -> Result<(), Error>
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

    if let Some(metrics_addr) = metrics_addr {
        let metrics_builder = PrometheusBuilder::new();

        metrics_builder
            .with_http_listener(metrics_addr)
            .install()
            .expect("failed to install prometheus metrics exporter");
    }

    Ok(registry()
        .with(stdout_log)
        .with(json_log)
        .with(env_filter)
        .try_init()?)
}
