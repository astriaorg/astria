use tokio::sync::watch;

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
    [set_sequencer_connected <- bool],
    [set_latest_confirmed_celestia_height <- u64],
    [set_latest_fetched_sequencer_height <- u64],
    [set_latest_observed_sequencer_height <- u64],
    [set_latest_requested_sequencer_height <- u64],
);

macro_rules! forward_getter {
    ($([$fn:ident -> $ret:ty]),*$(,)?) => {
        impl State {
            $(
            pub(super) fn $fn(&self) -> Option<$ret> {
                self.inner
                    .borrow()
                    .$fn()
            }
            )*
        }
    };
}

forward_getter!(
    [get_latest_confirmed_celestia_height -> u64],
);

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct StateSnapshot {
    ready: bool,

    celestia_connected: bool,
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

    fn get_latest_confirmed_celestia_height(&self) -> Option<u64> {
        self.latest_confirmed_celestia_height
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
    /// Returns if the previous state was modified.
    fn set_celestia_connected(&mut self, connected: bool) -> bool {
        let changed = self.celestia_connected ^ connected;
        self.celestia_connected = connected;
        changed
    }

    /// Sets the sequencer connected state to `connected`.
    ///
    /// Returns if the previous state was modified.
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
