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

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_sequence_action_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(keys::SEQUENCE_ACTION_BASE_FEE)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence action base fee from state")?
            .ok_or_eyre("sequence action base fee not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .wrap_err("invalid sequence action base fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_sequence_action_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(keys::SEQUENCE_ACTION_BYTE_COST_MULTIPLIER)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence action byte cost multiplier from state")?
            .ok_or_eyre("sequence action byte cost multiplier not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .wrap_err("invalid sequence action byte cost multiplier bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sequence_action_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::from(storage::Fee::from(fee))
            .serialize()
            .context("failed to serialize sequence action base fee")?;
        self.put_raw(keys::SEQUENCE_ACTION_BASE_FEE.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_sequence_action_byte_cost_multiplier(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::from(storage::Fee::from(fee))
            .serialize()
            .context("failed to serialize sequence action byte cost multiplier")?;
        self.put_raw(
            keys::SEQUENCE_ACTION_BYTE_COST_MULTIPLIER.to_string(),
            bytes,
        );
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use cnidarium::StateDelta;

    use super::*;

    #[tokio::test]
    async fn sequence_action_base_fee() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee = 42;
        state.put_sequence_action_base_fee(fee).unwrap();
        assert_eq!(state.get_sequence_action_base_fee().await.unwrap(), fee);
    }

    #[tokio::test]
    async fn sequence_action_byte_cost_multiplier() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee = 42;
        state.put_sequence_action_byte_cost_multiplier(fee).unwrap();
        assert_eq!(
            state
                .get_sequence_action_byte_cost_multiplier()
                .await
                .unwrap(),
            fee
        );
    }
}
