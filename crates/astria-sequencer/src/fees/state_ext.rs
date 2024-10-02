use astria_core::primitive::v1::{
    asset,
    TransactionId,
};
use astria_eyre::eyre::Result;
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::Fee;

const BLOCK_FEES_PREFIX: &str = "block_fees";

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    fn get_block_fees(&self) -> Result<Vec<Fee>> {
        let mut block_fees = self.object_get(BLOCK_FEES_PREFIX);
        match block_fees {
            Some(_) => {}
            None => {
                block_fees = Some(vec![]);
            }
        }
        Ok(block_fees.expect("block fees should not be `None` after populating"))
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    /// Constructs and adds `Fee` object to the block fees vec.
    #[instrument(skip_all)]
    fn add_fee_to_block_fees<'a, TAsset>(
        &mut self,
        asset: &'a TAsset,
        amount: u128,
        source_transaction_id: TransactionId,
        source_action_index: u64,
    ) -> Result<()>
    where
        TAsset: Sync + std::fmt::Display,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let current_fees: Option<Vec<Fee>> = self.object_get(BLOCK_FEES_PREFIX);

        let fee = Fee {
            asset: asset::IbcPrefixed::from(asset).into(),
            amount,
            source_transaction_id,
            source_action_index,
        };
        let new_fees = if let Some(mut fees) = current_fees {
            fees.push(fee);
            fees
        } else {
            vec![fee]
        };

        self.object_put(BLOCK_FEES_PREFIX, new_fees);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::primitive::v1::TransactionId;
    use cnidarium::StateDelta;

    use crate::fees::{
        Fee,
        StateReadExt as _,
        StateWriteExt as _,
    };

    fn asset_0() -> astria_core::primitive::v1::asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> astria_core::primitive::v1::asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn block_fee_read_and_increase() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // doesn't exist at first
        let fee_balances_orig = state.get_block_fees().unwrap();
        assert!(fee_balances_orig.is_empty());

        // can write
        let asset = asset_0();
        let amount = 100u128;
        state
            .add_fee_to_block_fees(&asset, amount, TransactionId::new([0; 32]), 0)
            .unwrap();

        // holds expected
        let fee_balances_updated = state.get_block_fees().unwrap();
        assert_eq!(
            fee_balances_updated[0],
            Fee {
                asset: asset.to_ibc_prefixed().into(),
                amount,
                source_transaction_id: TransactionId::new([0; 32]),
                source_action_index: 0
            },
            "fee balances are not what they were expected to be"
        );
    }

    #[tokio::test]
    async fn block_fee_read_and_increase_can_delete() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let asset_first = asset_0();
        let asset_second = asset_1();
        let amount_first = 100u128;
        let amount_second = 200u128;

        state
            .add_fee_to_block_fees(&asset_first, amount_first, TransactionId::new([0; 32]), 0)
            .unwrap();
        state
            .add_fee_to_block_fees(&asset_second, amount_second, TransactionId::new([0; 32]), 1)
            .unwrap();
        // holds expected
        let fee_balances = HashSet::<_>::from_iter(state.get_block_fees().unwrap());
        assert_eq!(
            fee_balances,
            HashSet::from_iter(vec![
                Fee {
                    asset: asset_first.to_ibc_prefixed().into(),
                    amount: amount_first,
                    source_transaction_id: TransactionId::new([0; 32]),
                    source_action_index: 0
                },
                Fee {
                    asset: asset_second.to_ibc_prefixed().into(),
                    amount: amount_second,
                    source_transaction_id: TransactionId::new([0; 32]),
                    source_action_index: 1
                },
            ]),
            "returned fee balance vector not what was expected"
        );
    }
}
