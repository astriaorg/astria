use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage;
use crate::storage::StoredValue;

const SEQUENCE_BASE_FEE_STORAGE_KEY: &str = "seqbasefee";
const SEQUENCE_COMPUTED_COST_MULTIPLIER_STORAGE_KEY: &str = "seqmultiplier";

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    
}
