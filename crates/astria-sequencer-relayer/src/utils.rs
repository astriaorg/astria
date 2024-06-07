use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::task::JoinError;

pub(crate) fn flatten<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}
