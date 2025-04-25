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
    InstrumentationScope,
};
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::TracerProvider,
};
use tracing_subscriber::{
    filter::{
        LevelFilter,
        ParseError,
    },
    fmt::{
        format::FmtSpan,
        writer::BoxMakeWriter,
        MakeWriter,
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

pub struct Config {
    filter_directives: String,
    force_stdout: bool,
    no_otel: bool,
    stdout_writer: BoxMakeWriter,
    metrics_config_builder: Option<metrics::ConfigBuilder>,
}

impl Config {
    #[must_use = "telemetry must be initialized to be useful"]
    fn new() -> Self {
        Self {
            filter_directives: String::new(),
            force_stdout: false,
            no_otel: false,
            stdout_writer: BoxMakeWriter::new(std::io::stdout),
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
    pub fn set_metrics(mut self, listening_addr: &str, service_name: &str) -> Self {
        let config_builder = metrics::ConfigBuilder::new()
            .set_service_name(service_name)
            .set_listening_address(listening_addr);
        self.metrics_config_builder = Some(config_builder);
        self
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_stdout_writer<M>(mut self, stdout_writer: M) -> Self
    where
        M: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    {
        self.stdout_writer = BoxMakeWriter::new(stdout_writer);
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
            let otel_exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()
                .map_err(Error::otlp)?;
            tracer_provider = tracer_provider.with_batch_exporter(otel_exporter, Tokio);
        }

        let mut formatter = None;
        if force_stdout || std::io::stdout().is_terminal() {
            formatter = Some(
                tracing_subscriber::fmt::layer()
                    .compact()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_writer(stdout_writer),
            );
        }
        let tracer_provider = tracer_provider.build();

        let tracer = tracer_provider.tracer_with_scope(
            InstrumentationScope::builder("astria_telemetry")
                .with_version(env!("CARGO_PKG_VERSION"))
                .with_schema_url(opentelemetry_semantic_conventions::SCHEMA_URL)
                .build(),
        );
        let _ = global::set_tracer_provider(tracer_provider);

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_subscriber::registry()
            .with(otel_layer)
            .with(formatter)
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
