use is_terminal::IsTerminal as _;
use tracing_subscriber::{
    filter::{
        EnvFilter,
        LevelFilter,
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

/// Register a global tracing subscriber.
///
/// # Errors
///
/// Returns the same errors as [`SubscriberInitExt::try_init`] called
/// on a [`tracing_subscriber::Registry`].
pub fn init<S>(sink: S) -> Result<(), TryInitError>
where
    S: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = init_env_filter();
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

    registry()
        .with(stdout_log)
        .with(json_log)
        .with(env_filter)
        .try_init()
}

fn init_env_filter() -> EnvFilter {
    let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
    match builder.try_from_env() {
        Err(e) => {
            eprintln!(
                "encountered invalid filter directives when setting up env filter for telemetry; \
                 continuing with default directive. Error while parsing: {e:?}"
            );
            builder.from_env_lossy()
        }
        Ok(env_filter) => env_filter,
    }
}
