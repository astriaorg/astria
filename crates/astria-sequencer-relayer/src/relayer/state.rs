use tokio::sync::watch;

use super::CelestiaCostParams;

pub(super) struct State {
    inner: watch::Sender<StateSnapshot>,
}

impl State {
    pub(super) fn new() -> Self {
        let (inner, _) = watch::channel(StateSnapshot::default());
        Self {
            inner,
        }
    }

    pub(super) fn set_ready(&self) {
        self.inner.send_modify(StateSnapshot::set_ready);
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<StateSnapshot> {
        self.inner.subscribe()
    }

    pub(super) fn celestia_cost_params(&self) -> CelestiaCostParams {
        self.inner.borrow().celestia_cost_params
    }
}

macro_rules! forward_setter {
    ($([$fn:ident <- $val:ty]),*$(,)?) => {
        impl State {
            $(
            pub(super) fn $fn(&self, val: $val) {
                self.inner
                    .send_if_modified(|state| state.$fn(val));
            }
            )*
        }
    };
}

forward_setter!(
    [set_celestia_connected <- bool],
    [set_celestia_cost_params <- CelestiaCostParams],
    [set_sequencer_connected <- bool],
    [set_latest_confirmed_celestia_height <- u64],
    [set_latest_fetched_sequencer_height <- u64],
    [set_latest_observed_sequencer_height <- u64],
    [set_latest_requested_sequencer_height <- u64],
);

#[derive(Copy, Clone, Debug, Default, PartialEq, serde::Serialize)]
pub(crate) struct StateSnapshot {
    ready: bool,

    celestia_connected: bool,
    celestia_cost_params: CelestiaCostParams,
    sequencer_connected: bool,

    latest_confirmed_celestia_height: Option<u64>,

    latest_fetched_sequencer_height: Option<u64>,
    latest_observed_sequencer_height: Option<u64>,
    latest_requested_sequencer_height: Option<u64>,
}

impl StateSnapshot {
    fn set_ready(&mut self) {
        self.ready = true;
    }

    fn set_latest_confirmed_celestia_height(&mut self, height: u64) -> bool {
        let changed = self
            .latest_confirmed_celestia_height
            .map_or(true, |h| h != height);
        self.latest_confirmed_celestia_height.replace(height);
        changed
    }

    fn set_latest_fetched_sequencer_height(&mut self, height: u64) -> bool {
        let changed = self
            .latest_fetched_sequencer_height
            .map_or(true, |h| h != height);
        self.latest_fetched_sequencer_height.replace(height);
        changed
    }

    fn set_latest_observed_sequencer_height(&mut self, height: u64) -> bool {
        let changed = self
            .latest_observed_sequencer_height
            .map_or(true, |h| h != height);
        self.latest_observed_sequencer_height.replace(height);
        changed
    }

    fn set_latest_requested_sequencer_height(&mut self, height: u64) -> bool {
        let changed = self
            .latest_requested_sequencer_height
            .map_or(true, |h| h != height);
        self.latest_requested_sequencer_height.replace(height);
        changed
    }

    /// Sets the celestia connected state to `connected`.
    ///
    /// Returns `true` if the previous state was modified.
    fn set_celestia_connected(&mut self, connected: bool) -> bool {
        let changed = self.celestia_connected ^ connected;
        self.celestia_connected = connected;
        changed
    }

    /// Sets the celestia cost params.
    ///
    /// Returns `true` if the previous state was modified.
    fn set_celestia_cost_params(&mut self, cost_params: CelestiaCostParams) -> bool {
        let changed = self.celestia_cost_params != cost_params;
        self.celestia_cost_params = cost_params;
        changed
    }

    /// Sets the sequencer connected state to `connected`.
    ///
    /// Returns `true` if the previous state was modified.
    fn set_sequencer_connected(&mut self, connected: bool) -> bool {
        let changed = self.sequencer_connected ^ connected;
        self.sequencer_connected = connected;
        changed
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.celestia_connected && self.sequencer_connected
    }
}
