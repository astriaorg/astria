pub(crate) mod executor;
pub(crate) mod geth_collector;
pub(crate) mod rollup;

type StdError = dyn std::error::Error;

/// Announces the current status of the Searcher for other modules in the crate to use
#[derive(Debug, Default)]
pub(crate) struct Status {
    pub(crate) all_collectors_connected: bool,
    pub(crate) executor_connected: bool,
}

impl Status {
    pub(crate) fn is_ready(&self) -> bool {
        self.all_collectors_connected && self.executor_connected
    }
}