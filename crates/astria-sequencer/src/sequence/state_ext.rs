use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use crate::storage::verifiable_keys::sequence::{
    SEQUENCE_ACTION_BASE_FEE_KEY,
    SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_KEY,
};

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Fee(u128);

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_sequence_action_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(SEQUENCE_ACTION_BASE_FEE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence action base fee from state")?
            .ok_or_eyre("sequence action base fee not found")?;
        let Fee(fee) = Fee::try_from_slice(&bytes).wrap_err("invalid fee bytes")?;
        Ok(fee)
    }

    #[instrument(skip_all)]
    async fn get_sequence_action_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence action byte cost multiplier from state")?
            .ok_or_eyre("sequence action byte cost multiplier not found")?;
        let Fee(fee) = Fee::try_from_slice(&bytes).wrap_err("invalid fee bytes")?;
        Ok(fee)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sequence_action_base_fee(&mut self, fee: u128) {
        self.put_raw(
            SEQUENCE_ACTION_BASE_FEE_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).expect("failed to serialize fee"),
        );
    }

    #[instrument(skip_all)]
    fn put_sequence_action_byte_cost_multiplier(&mut self, fee: u128) {
        self.put_raw(
            SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).expect("failed to serialize fee"),
        );
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn sequence_action_base_fee() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee = 42;
        state.put_sequence_action_base_fee(fee);
        assert_eq!(state.get_sequence_action_base_fee().await.unwrap(), fee);
    }

    #[tokio::test]
    async fn sequence_action_byte_cost_multiplier() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee = 42;
        state.put_sequence_action_byte_cost_multiplier(fee);
        assert_eq!(
            state
                .get_sequence_action_byte_cost_multiplier()
                .await
                .unwrap(),
            fee
        );
    }
}
