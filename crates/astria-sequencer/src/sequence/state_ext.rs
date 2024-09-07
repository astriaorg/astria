use anyhow::{
    anyhow,
    Context,
    Result,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use crate::storage::{
    self,
    StoredValue,
};

const SEQUENCE_ACTION_BASE_FEE_STORAGE_KEY: &str = "seqbasefee";
const SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_STORAGE_KEY: &str = "seqmultiplier";

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_sequence_action_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(SEQUENCE_ACTION_BASE_FEE_STORAGE_KEY)
            .await
            .context("failed reading raw sequence action base fee from state")?
            .ok_or_else(|| anyhow!("sequence action base fee not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid sequence action base fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_sequence_action_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_STORAGE_KEY)
            .await
            .context("failed reading raw sequence action byte cost multiplier from state")?
            .ok_or_else(|| anyhow!("sequence action byte cost multiplier not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid sequence action byte cost multiplier bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sequence_action_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::Fee(fee.into())
            .serialize()
            .context("failed to serialize sequence action base fee")?;
        self.put_raw(SEQUENCE_ACTION_BASE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_sequence_action_byte_cost_multiplier(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::Fee(fee.into())
            .serialize()
            .context("failed to serialize sequence action byte cost multiplier")?;
        self.put_raw(
            SEQUENCE_ACTION_BYTE_COST_MULTIPLIER_STORAGE_KEY.to_string(),
            bytes,
        );
        Ok(())
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
