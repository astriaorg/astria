/// Announces the current status of the Composer for other modules in the crate to use
#[derive(Debug, Default)]
pub(super) struct ComposerStatus {
    all_collectors_connected: bool,
    executor_connected: bool,
}

impl ComposerStatus {
    pub(super) fn is_ready(&self) -> bool {
        self.all_collectors_connected && self.executor_connected
    }

    pub(super) fn set_all_collectors_connected(&mut self, connected: bool) {
        self.all_collectors_connected = connected;
    }

    pub(super) fn set_executor_connected(&mut self, connected: bool) {
        self.executor_connected = connected;
    }
}
