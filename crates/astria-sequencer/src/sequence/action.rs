use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::SequenceAction,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    sequence::state_ext::StateReadExt as SequenceStateReadExt,
    state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for SequenceAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ensure!(
            state.is_allowed_fee_asset(self.fee_asset_id).await?,
            "invalid fee asset",
        );

        let curr_balance = state
            .get_account_balance(from, self.fee_asset_id)
            .await
            .context("failed getting `from` account balance for fee payment")?;
        let fee = calculate_fee_from_state(&self.data, state)
            .await
            .context("calculated fee overflows u128")?;
        ensure!(curr_balance >= fee, "insufficient funds");
        Ok(())
    }

    async fn check_stateless(&self) -> Result<()> {
        // TODO: do we want to place a maximum on the size of the data?
        // https://github.com/astriaorg/astria/issues/222
        ensure!(
            !self.data.is_empty(),
            "cannot have empty data for sequence action"
        );
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            from = from.to_string(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let fee = calculate_fee_from_state(&self.data, state)
            .await
            .context("failed to calculate fee")?;
        state
            .get_and_increase_block_fees(self.fee_asset_id, fee)
            .await
            .context("failed to add to block fees")?;

        state
            .decrease_balance(from, self.fee_asset_id, fee)
            .await
            .context("failed updating `from` account balance")?;
        Ok(())
    }
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
pub(crate) async fn calculate_fee_from_state<S: SequenceStateReadExt>(
    data: &[u8],
    state: &S,
) -> Result<u128> {
    let base_fee = state
        .get_sequence_action_base_fee()
        .await
        .context("failed to get base fee")?;
    let fee_per_byte = state
        .get_sequence_action_byte_cost_multiplier()
        .await
        .context("failed to get fee per byte")?;
    calculate_fee(data, fee_per_byte, base_fee).context("calculated fee overflows u128")
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
/// Returns `None` if the fee overflows `u128`.
fn calculate_fee(data: &[u8], fee_per_byte: u128, base_fee: u128) -> Option<u128> {
    base_fee.checked_add(
        fee_per_byte.checked_mul(
            data.len()
                .try_into()
                .expect("a usize should always convert to a u128"),
        )?,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculate_fee_ok() {
        assert_eq!(calculate_fee(&[], 1, 0), Some(0));
        assert_eq!(calculate_fee(&[0], 1, 0), Some(1));
        assert_eq!(calculate_fee(&[0u8; 10], 1, 0), Some(10));
        assert_eq!(calculate_fee(&[0u8; 10], 1, 100), Some(110));
    }
}
