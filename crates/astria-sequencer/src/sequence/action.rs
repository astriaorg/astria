use astria_core::protocol::transaction::v1alpha1::action::SequenceAction;
use astria_eyre::eyre::{
    ensure,
    Result,
};
use cnidarium::StateWrite;

use crate::app::ActionHandler;

#[async_trait::async_trait]
impl ActionHandler for SequenceAction {
    async fn check_stateless(&self) -> Result<()> {
        // TODO: do we want to place a maximum on the size of the data?
        // https://github.com/astriaorg/astria/issues/222
        ensure!(
            !self.data.is_empty(),
            "cannot have empty data for sequence action"
        );
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
/// Returns `None` if the fee overflows `u128`.
pub(crate) fn calculate_fee(data: &[u8], fee_per_byte: u128, base_fee: u128) -> Option<u128> {
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
