use tokio::sync::watch;

pub(crate) struct State {
    inner: tokio::sync::watch::Sender<StateSnapshot>,
}

impl State {
    pub(super) fn new() -> Self {
        let (inner, _) = watch::channel(StateSnapshot::default());
        Self {
            inner,
        }
    }

    pub(super) fn set_watcher_ready(&self) {
        self.inner.send_modify(StateSnapshot::set_watcher_ready);
    }

    pub(super) fn set_submitter_ready(&self) {
        self.inner.send_modify(StateSnapshot::set_submitter_ready);
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
    [set_sequencer_connected <- bool],
    [set_last_rollup_height_submitted <- u64],
    [set_last_sequencer_height <- u64],
    [set_last_sequencer_tx_hash <- tendermint::Hash],
);

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct StateSnapshot {
    watcher_ready: bool,
    submitter_ready: bool,

    sequencer_connected: bool,

    last_rollup_height_submitted: Option<u64>,
    last_sequencer_block: Option<u64>,
    last_sequencer_tx_hash: Option<tendermint::Hash>,
}

impl StateSnapshot {
    pub(crate) fn set_watcher_ready(&mut self) {
        self.watcher_ready = true;
    }

    pub(crate) fn set_submitter_ready(&mut self) {
        self.submitter_ready = true;
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.submitter_ready && self.watcher_ready
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.sequencer_connected
    }

    /// Sets the sequencer connection status to `connected`.
    fn set_sequencer_connected(&mut self, connected: bool) -> bool {
        let changed = self.sequencer_connected ^ connected;
        self.sequencer_connected = connected;
        changed
    }

    fn set_last_rollup_height_submitted(&mut self, height: u64) -> bool {
        let changed = self
            .last_rollup_height_submitted
            .map_or(true, |h| h != height);
        self.last_rollup_height_submitted = Some(height);
        changed
    }

    fn set_last_sequencer_height(&mut self, height: u64) -> bool {
        let changed = self.last_sequencer_block.map_or(true, |h| h != height);
        self.last_sequencer_block = Some(height);
        changed
    }

    fn set_last_sequencer_tx_hash(&mut self, hash: tendermint::Hash) -> bool {
        let changed = self.last_sequencer_tx_hash.map_or(true, |h| h != hash);
        self.last_sequencer_tx_hash = Some(hash);
        changed
    }
}
