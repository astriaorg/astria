use tokio::sync::watch;

pub(super) struct State {
    inner: tokio::sync::watch::Sender<StateSnapshot>,
}

impl State {
    pub(super) fn new() -> Self {
        let (inner, _) = watch::channel(StateSnapshot::default());
        Self {
            inner,
        }
    }

    pub(super) fn set_ready(&self) {
        self.inner.send_modify(|_| ());
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<StateSnapshot> {
        self.inner.subscribe()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct StateSnapshot {
    ready: bool,
}

impl StateSnapshot {
    pub(crate) fn set_ready(&mut self) {
        self.ready = true;
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready
    }

    pub(crate) fn is_healthy(&self) -> bool {
        todo!()
    }
}
