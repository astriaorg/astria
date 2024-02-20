//! Initialize telemetry in all astria services.
//!
//! # Examples
//! ```no_run
//! astria_telemetry::configure()
//!     .filter_directives("info")
//!     .try_init()
//!     .expect("must be able to initialize telemetry");
//! tracing::info!("telemetry initialized");
//! ```
use std::{
    io::IsTerminal as _,
    net::{
        AddrParseError,
        SocketAddr,
    },
};

use metrics_exporter_prometheus::{
    BuildError,
    PrometheusBuilder,
};
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

/// The errors that can occur when initializing telemtry.
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

    fn metrics_addr(source: AddrParseError) -> Self {
        Self(ErrorKind::MetricsAddr(source))
    }

    fn bucket_error(source: BuildError) -> Self {
        Self(ErrorKind::BucketError(source))
    }

    fn exporter_install(source: BuildError) -> Self {
        Self(ErrorKind::ExporterInstall(source))
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
    #[error("failed to parse metrics address")]
    MetricsAddr(#[source] AddrParseError),
    #[error("failed to configure prometheus buckets")]
    BucketError(#[source] BuildError),
    #[error("failed installing prometheus metrics exporter")]
    ExporterInstall(#[source] BuildError),
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
    metrics_addr: Option<String>,
    service_name: String,
    metric_buckets: Option<Vec<f64>>,
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
            metrics_addr: None,
            service_name: String::new(),
            metric_buckets: None,
        }
    }
}

impl Config {
    #[must_use = "telemetry must be initialized to be useful"]
    pub fn filter_directives(self, filter_directives: &str) -> Self {
        Self {
            filter_directives: filter_directives.to_string(),
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn force_stdout(self) -> Self {
        self.set_force_stdout(true)
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_force_stdout(self, force_stdout: bool) -> Self {
        Self {
            force_stdout,
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn no_otel(self) -> Self {
        self.set_no_otel(true)
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_no_otel(self, no_otel: bool) -> Self {
        Self {
            no_otel,
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn pretty_print(self) -> Self {
        self.set_pretty_print(true)
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn set_pretty_print(self, pretty_print: bool) -> Self {
        Self {
            pretty_print,
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn stdout_writer<M>(self, stdout_writer: M) -> Self
    where
        M: MakeWriter + Send + Sync + 'static,
    {
        Self {
            stdout_writer: BoxedMakeWriter::new(stdout_writer),
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn metrics_addr(self, metrics_addr: &str) -> Self {
        Self {
            metrics_addr: Some(metrics_addr.to_string()),
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn service_name(self, service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            ..self
        }
    }

    #[must_use = "telemetry must be initialized to be useful"]
    pub fn metric_buckets(self, metric_buckets: Vec<f64>) -> Self {
        Self {
            metric_buckets: Some(metric_buckets),
            ..self
        }
    }

    /// Initialize telemetry, consuming the config.
    ///
    /// # Errors
    /// Fails if the filter directives could not be parsed, if communication with the OTLP
    /// endpoint failed, or if the global tracing subscriber could not be installed.
    pub fn try_init(self) -> Result<(), Error> {
        let Self {
            filter_directives,
            force_stdout,
            no_otel,
            pretty_print,
            stdout_writer,
            metrics_addr,
            service_name,
            metric_buckets,
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
            //      full list of variables that opentelementry_otlp currently reads:
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

        if let Some(metrics_addr) = metrics_addr {
            let addr: SocketAddr = metrics_addr.parse().map_err(Error::metrics_addr)?;
            let mut metrics_builder = PrometheusBuilder::new().with_http_listener(addr);

            if !service_name.is_empty() {
                metrics_builder = metrics_builder.add_global_label("service", service_name);
            }

            if let Some(buckets) = metric_buckets {
                metrics_builder = metrics_builder
                    .set_buckets(&buckets)
                    .map_err(Error::bucket_error)?;
            }

            metrics_builder.install().map_err(Error::exporter_install)?;
        }

        Ok(())
    }
}
