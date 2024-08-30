//! Initialize telemetry in all astria services.
//!
//! # Examples
//! ```no_run
//! # struct Metrics;
//! # impl astria_telemetry::metrics::Metrics for Metrics {
//! #     type Config = ();
//! #     fn register(
//! #         _: &mut astria_telemetry::metrics::RegisteringBuilder,
//! #         _: &Self::Config
//! #     ) -> Result<Self, astria_telemetry::metrics::Error> { Ok(Self) }
//! # }
//! let metrics_config = ();
//! astria_telemetry::configure()
//!     .set_filter_directives("info")
//!     .try_init::<Metrics>(&metrics_config)
//!     .expect("must be able to initialize telemetry");
//! tracing::info!("telemetry initialized");
//! ```
use std::io::IsTerminal as _;

pub use metrics::Metrics;
use opentelemetry::{
    global,
    trace::TracerProvider as _,
};
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::TracerProvider,
};
use opentelemetry_stdout::SpanExporter;
use tracing_subscriber::{
    filter::{
        LevelFilter,
        ParseError,
    },
    layer::SubscriberExt as _,
    util::{
        SubscriberInitExt as _,
        TryInitError,
    },
    EnvFilter,
};

#[cfg(feature = "display")]
pub mod display;
#[doc(hidden)]
pub mod macros;
pub mod metrics;

/// The errors that can occur when initializing telemetry.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn otlp(source: opentelemetry::trace::TraceError) -> Self {
        Self(ErrorKind::Otlp(source))
    }

    fn filter_directives(source: ParseError) -> Self {
        Self(ErrorKind::FilterDirectives(source))
    }

    fn init_subscriber(source: TryInitError) -> Self {
        Self(ErrorKind::InitSubscriber(source))
    }
}

impl From<metrics::Error> for Error {
    fn from(source: metrics::Error) -> Self {
        Self(ErrorKind::Metrics(source))
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("failed constructing opentelemetry otlp exporter")]
    Otlp(#[source] opentelemetry::trace::TraceError),
    #[error("failed to parse filter directives")]
    FilterDirectives(#[source] ParseError),
    #[error("failed installing global tracing subscriber")]
    InitSubscriber(#[source] TryInitError),
    #[error(transparent)]
    Metrics(#[from] metrics::Error),
}

#[must_use = "the otel config must be initialized to be useful"]
pub fn configure() -> Config {
    Config::new()
}

struct BoxedMakeWriter(Box<dyn MakeWriter + Send + Sync + 'static>);

impl BoxedMakeWriter {
    fn new<M>(make_writer: M) -> Self
    where
        M: MakeWriter + Send + Sync + 'static,
    {
        Self(Box::new(make_writer))
    }
}

pub trait MakeWriter {
    fn make_writer(&self) -> Box<dyn std::io::Write + Send + Sync + 'static>;
}

impl<F, W> MakeWriter for F
where
    F: Fn() -> W,
    W: std::io::Write + Send + Sync + 'static,
{
    fn make_writer(&self) -> Box<dyn std::io::Write + Send + Sync + 'static> {
        Box::new((self)())
    }
}

impl MakeWriter for BoxedMakeWriter {
    fn make_writer(&self) -> Box<dyn std::io::Write + Send + Sync + 'static> {
        self.0.make_writer()
    }
}

pub struct Config {
    filter_directives: String,
    force_stdout: bool,
    no_otel: bool,
    pretty_print: bool,
    stdout_writer: BoxedMakeWriter,
    metrics_config_builder: Option<metrics::ConfigBuilder>,
}

impl Config {
    #[must_use = "telemetry must be initialized to be useful"]
    fn new() -> Self {
        Self {
            filter_directives: String::new(),
            force_stdout: false,
            no_otel: false,
            pretty_print: false,
            stdout_writer: BoxedMakeWriter::new(std::io::stdout),
            metrics_config_builder: None,
        }
    }
}

impl Config {
    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_filter_directives(mut self, filter_directives: &str) -> Self {
        self.filter_directives = filter_directives.to_string();
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_force_stdout(mut self, force_stdout: bool) -> Self {
        self.force_stdout = force_stdout;
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_no_otel(mut self, no_otel: bool) -> Self {
        self.no_otel = no_otel;
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_pretty_print(mut self, pretty_print: bool) -> Self {
        self.pretty_print = pretty_print;
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_stdout_writer<M>(mut self, stdout_writer: M) -> Self
    where
        M: MakeWriter + Send + Sync + 'static,
    {
        self.stdout_writer = BoxedMakeWriter::new(stdout_writer);
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_metrics(mut self, listening_addr: &str, service_name: &str) -> Self {
        let config_builder = metrics::ConfigBuilder::new()
            .set_service_name(service_name)
            .set_listening_address(listening_addr);
        self.metrics_config_builder = Some(config_builder);
        self
    }

    /// Initialize telemetry, consuming the config.
    ///
    /// # Errors
    /// Fails if the filter directives could not be parsed, if communication with the OTLP
    /// endpoint failed, or if the global tracing subscriber could not be installed.
    pub fn try_init<T: Metrics>(self, config: &T::Config) -> Result<(&'static T, Guard), Error> {
        let Self {
            filter_directives,
            force_stdout,
            no_otel,
            pretty_print,
            stdout_writer,
            metrics_config_builder,
        } = self;

        let env_filter = {
            let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
            builder
                .parse(filter_directives)
                .map_err(Error::filter_directives)?
        };

        let mut tracer_provider = TracerProvider::builder();
        if !no_otel {
            // XXX: the endpoint is set by a hardcoded environment variable. This is a
            //      full list of variables that opentelemetry_otlp currently reads:
            //      OTEL_EXPORTER_OTLP_ENDPOINT
            //      OTEL_EXPORTER_OTLP_TRACES_ENDPOINT
            //      OTEL_EXPORTER_OTLP_TRACES_TIMEOUT
            //      OTEL_EXPORTER_OTLP_TRACES_COMPRESSION
            //      OTEL_EXPORTER_OTLP_HEADERS
            //      OTEL_EXPORTER_OTLP_TRACE_HEADERS
            let otel_exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .build_span_exporter()
                .map_err(Error::otlp)?;
            tracer_provider = tracer_provider.with_batch_exporter(otel_exporter, Tokio);
        }

        let mut pretty_printer = None;
        if force_stdout || std::io::stdout().is_terminal() {
            if pretty_print {
                pretty_printer = Some(tracing_subscriber::fmt::layer().compact());
            } else {
                tracer_provider = tracer_provider.with_simple_exporter(
                    SpanExporter::builder()
                        .with_writer(stdout_writer.make_writer())
                        .build(),
                );
            }
        }
        let tracer_provider = tracer_provider.build();

        let tracer = tracer_provider.versioned_tracer(
            "astria-telemetry",
            Some(env!("CARGO_PKG_VERSION")),
            Some(opentelemetry_semantic_conventions::SCHEMA_URL),
            None,
        );
        let _ = global::set_tracer_provider(tracer_provider);

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_subscriber::registry()
            .with(otel_layer)
            .with(pretty_printer)
            .with(env_filter)
            .try_init()
            .map_err(Error::init_subscriber)?;

        let metrics = match metrics_config_builder {
            Some(config_builder) => config_builder.build(config)?.0,
            None => T::noop_metrics(config)?,
        };

        let guard = Guard {
            run_otel_shutdown: !no_otel,
        };

        Ok((Box::leak(Box::new(metrics)), guard))
    }
}

/// A drop guard for terminating all `OpenTelemetry` tracer providers on drop.
///
/// *Note:* Shutting down the tracer providers can potentially block a thread
/// indefinitely.
pub struct Guard {
    run_otel_shutdown: bool,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if self.run_otel_shutdown {
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}
