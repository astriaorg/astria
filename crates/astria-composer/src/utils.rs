use astria_eyre::eyre::Report;
use tracing::{
    error,
    info,
    instrument,
};

#[instrument(skip_all)]
pub(crate) fn report_exit_reason(reason: &Result<&str, Report>) {
    match &reason {
        Ok(reason) => {
            info!(reason, "shutting down");
        }
        Err(reason) => {
            error!(%reason, "shutting down");
        }
    }
}
