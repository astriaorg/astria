use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use celestia_client::celestia_types::Height as CelestiaHeight;
use sequencer_client::tendermint::block::Height as SequencerHeight;
use tokio::task::JoinError;

/// A necessary evil because the celestia client code uses a forked tendermint-rs.
pub(crate) trait IncrementableHeight {
    fn increment(self) -> Self;
}

impl IncrementableHeight for CelestiaHeight {
    fn increment(self) -> Self {
        self.increment()
    }
}

impl IncrementableHeight for SequencerHeight {
    fn increment(self) -> Self {
        self.increment()
    }
}

pub(crate) fn flatten<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
