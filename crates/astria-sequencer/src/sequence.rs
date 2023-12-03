use anyhow::{
    ensure,
    Context,
    Result,
};
use proto::native::sequencer::v1alpha1::{
    asset,
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
        fee_asset_id: asset::Id,
    ) -> Result<()> {
        let curr_balance = state
            .get_account_balance(from, fee_asset_id)
            .await
            .context("failed getting `from` account balance for fee payment")?;
        let fee = calculate_fee(self).context("calculating fee overflowed u128")?;
        ensure!(curr_balance >= fee, "insufficient funds");
        Ok(())
    }

    async fn check_stateless(&self) -> Result<()> {
        // TODO: do we want to place a maximum on the size of the data?
        // https://github.com/astriaorg/astria/issues/222

        // XXX(superfluffy): I have removed the check for data being empty/containing
        // no bytes. As all sequence actions now cost money due to the roll ID being priced
        // in users are free to submit as many empty transactions as desired.
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            from = from.to_string(),
        )
    )]
    async fn execute<S: StateWriteExt>(
        &self,
        state: &mut S,
        from: Address,
        fee_asset_id: asset::Id,
    ) -> Result<()> {
        let fee = calculate_fee(self).context("calculating fee overflowed u128")?;
        let from_balance = state
            .get_account_balance(from, fee_asset_id)
            .await
            .context("failed getting `from` account balance")?;
        state
            .put_account_balance(from, fee_asset_id, from_balance - fee)
            .context("failed updating `from` account balance")?;
        Ok(())
    }
}

/// Calculates the fee for submitting a sequence action to the sequencer.
///
/// The fee is calculated as:
/// ```ignore
/// bytes =  len(ID) + sum_i(len(tx_i)) + N
/// fee = FEE_PER_BYTE * bytes
/// ```
/// where `len(ID)` is the length of the rollup ID (32 bytes),
/// `sum_i(len(tx_i))` is the sum over the number of bytes per transaction `i`,
/// and `N` is the total number of transactions.
///
/// `N` is a naive heuristic for how many extra bytes protobuf will store for the
/// index of each additional transaction. Since these indices are encoded as varints
/// this heuristic breaks down for large N, but should be fine up to 255 transactions
/// or so.
///
/// Returns `None` if accumulating the total number of bytes or multiplying
/// the total number of bytes by the fee overflows `u128`.
pub(crate) fn calculate_fee(action: &SequenceAction) -> Option<u128> {
    // bytes = len(ID) + N
    let mut bytes = action
        .rollup_id()
        .len()
        .checked_add(action.transactions().len())?;

    // bytes += sum(len(tx_i))
    for tx in action.transactions() {
        bytes = bytes.checked_add(tx.len())?;
    }

    let bytes = bytes
        .try_into()
        .expect("usize converts to u128 on all currently existing machines");

    SEQUENCE_ACTION_FEE_PER_BYTE.checked_mul(bytes)
}

#[cfg(test)]
mod test {
    use super::calculate_fee;

    #[test]
    fn calculate_fee_ok() {
        assert_eq!(calculate_fee(&[]), Some(0));
        assert_eq!(calculate_fee(&[0]), Some(1));
        assert_eq!(calculate_fee(&[0u8; 10]), Some(10));
    }
}
