use color_eyre::eyre::{self, WrapErr as _};
use is_terminal::IsTerminal as _;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{self, MakeWriter},
    layer::SubscriberExt as _,
    registry,
    util::SubscriberInitExt as _,
};

/// Register a global tracing subscriber.
///
/// # Errors
///
/// Returns the same errors as [`SubscriberInitExt::try_init`] called
/// on a [`tracing_subscriber::Registry`].
pub fn init<S>(sink: S, filter_directives: &str) -> eyre::Result<()>
where
    S: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        init_env_filter(filter_directives).wrap_err("failed initializing telemetry env filter")?;
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
        .wrap_err("failed initializing telemetry stack")
}

fn init_env_filter(dirs: &str) -> eyre::Result<EnvFilter> {
    let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
    builder
        .parse(dirs)
        .wrap_err("failed parsing configured filter directives")
}
