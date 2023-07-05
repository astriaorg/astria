use is_terminal::IsTerminal as _;
use tracing_subscriber::{
    filter::{
        EnvFilter,
        ParseError,
    },
    fmt::{
        self,
        MakeWriter,
    },
    layer::SubscriberExt as _,
    registry,
    util::{
        SubscriberInitExt as _,
        TryInitError,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log directive")]
    InvalidLogDirective(#[from] ParseError),
    #[error("failed to initialize subscriber")]
    TryInitError(#[from] TryInitError),
}

/// Initialize the global tracing subscriber.
/// # Errors
/// Returns a `TryInitError` if the subscriber fails to initialize.
pub fn init<S>(log: &str, sink: S) -> Result<(), Error>
where
    S: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_new(log)?;
    let (json_log, stdout_log) = if std::io::stdout().is_terminal() {
        eprintln!("service is attached to tty; using human readable formatting");
        (None, Some(fmt::layer().pretty().with_writer(sink)))
    } else {
        eprintln!("service is not attached to tty; using json formatting");
        (
            Some(fmt::layer().json().flatten_event(true).with_writer(sink)),
            None,
        )
    };

    registry()
        .with(stdout_log)
        .with(json_log)
        .with(env_filter)
        .try_init()
        .map_err(Error::TryInitError)
}
