use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::{
    Address,
    SequenceAction,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

/// Fee charged for a sequence `Action` per byte of `data` included.
const SEQUENCE_ACTION_FEE_PER_BYTE: u128 = 1;

#[async_trait::async_trait]
impl ActionHandler for SequenceAction {
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        let curr_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        let fee = calculate_fee(&self.data).context("calculated fee overflows u128")?;
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
        let fee = calculate_fee(&self.data).context("failed to calculate fee")?;
        let from_balance = state
            .get_account_balance(from)
            .await
            .context("failed getting `from` account balance")?;
        state
            .put_account_balance(from, from_balance - fee)
            .context("failed updating `from` account balance")?;
        Ok(())
    }
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
/// Returns `None` if the fee overflows `u128`.
pub(crate) fn calculate_fee(data: &[u8]) -> Option<u128> {
    SEQUENCE_ACTION_FEE_PER_BYTE.checked_mul(
        data.len()
            .try_into()
            .expect("a usize should always convert to a u128"),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculate_fee_ok() {
        assert_eq!(calculate_fee(&[]), Some(0));
        assert_eq!(calculate_fee(&[0]), Some(1));
        assert_eq!(calculate_fee(&[0u8; 10]), Some(10));
    }
}
