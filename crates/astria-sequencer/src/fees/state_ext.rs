use astria_core::{
    primitive::v1::{
        asset,
        TransactionId,
    },
    protocol::transaction::v1alpha1::action::FeeComponents,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        eyre,
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

use super::{
    storage,
    Fee,
};
use crate::storage::StoredValue;

const BLOCK_FEES_PREFIX: &str = "block_fees";
const TRANSFER_FEES_STORAGE_KEY: &str = "transferfees";
const SEQUENCE_FEES_STORAGE_KEY: &str = "sequencefees";
const ICS20_WITHDRAWAL_FEES_STORAGE_KEY: &str = "ics20fees";
const INIT_BRIDGE_ACCOUNT_FEES_STORAGE_KEY: &str = "initbridgefees";
const BRIDGE_LOCK_FEES_STORAGE_KEY: &str = "bridgelockfees";
const BRIDGE_UNLOCK_FEES_STORAGE_KEY: &str = "bridgeunlockfees";
const BRIDGE_SUDO_CHANGE_FEES_STORAGE_KEY: &str = "bridgesudochangefees";

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

    #[instrument(skip_all)]
    async fn get_transfer_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(TRANSFER_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw transfer fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("transfer fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_sequence_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(SEQUENCE_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw sequence fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("sequence fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_ics20_withdrawal_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(ICS20_WITHDRAWAL_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ics20 withdrawal fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("ics20 withdrawal fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(INIT_BRIDGE_ACCOUNT_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw init bridge account fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("init bridge account fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(BRIDGE_LOCK_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge lock fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge lock fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_unlock_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(BRIDGE_UNLOCK_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge unlock fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge unlock fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_fees(&self) -> Result<FeeComponents> {
        let bytes = self
            .get_raw(BRIDGE_SUDO_CHANGE_FEES_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge sudo change fee components from state")?;
        let Some(bytes) = bytes else {
            return Err(eyre!("bridge sudo change fee components not set"));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::FeeComponents::try_from(value).map(FeeComponents::from))
            .wrap_err("invalid fees bytes")
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

    #[instrument(skip_all)]
    fn put_transfer_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(TRANSFER_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_sequence_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(SEQUENCE_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ics20_withdrawal_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(ICS20_WITHDRAWAL_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_init_bridge_account_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(INIT_BRIDGE_ACCOUNT_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_lock_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(BRIDGE_LOCK_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_unlock_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(BRIDGE_UNLOCK_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_sudo_change_fees(&mut self, fees: FeeComponents) -> Result<()> {
        let bytes = StoredValue::from(storage::FeeComponents::from(fees))
            .serialize()
            .wrap_err("failed to serialize fees")?;
        self.put_raw(BRIDGE_SUDO_CHANGE_FEES_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use astria_core::{
        primitive::v1::TransactionId,
        protocol::transaction::v1alpha1::action::{
            BridgeLockFeeComponents,
            BridgeSudoChangeFeeComponents,
            BridgeUnlockFeeComponents,
            FeeComponents,
            Ics20WithdrawalFeeComponents,
            InitBridgeAccountFeeComponents,
            SequenceFeeComponents,
            TransferFeeComponents,
        },
    };
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

    #[tokio::test]
    async fn transfer_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeComponents::TransferFeeComponents(TransferFeeComponents {
            base_fee: 123,
            computed_cost_multiplier: 1,
        });

        state.put_transfer_fees(fee_components).unwrap();
        let retrieved_fee = state.get_transfer_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn sequence_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeComponents::SequenceFeeComponents(SequenceFeeComponents {
            base_fee: 123,
            computed_cost_multiplier: 1,
        });

        state.put_sequence_fees(fee_components).unwrap();
        let retrieved_fee = state.get_sequence_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn init_bridge_account_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components =
            FeeComponents::InitBridgeAccountFeeComponents(InitBridgeAccountFeeComponents {
                base_fee: 123,
                computed_cost_multiplier: 1,
            });

        state.put_init_bridge_account_fees(fee_components).unwrap();
        let retrieved_fee = state.get_init_bridge_account_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn ics20_withdrawal_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components =
            FeeComponents::Ics20WithdrawalFeeComponents(Ics20WithdrawalFeeComponents {
                base_fee: 123,
                computed_cost_multiplier: 1,
            });

        state.put_ics20_withdrawal_fees(fee_components).unwrap();
        let retrieved_fee = state.get_ics20_withdrawal_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_lock_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeComponents::BridgeLockFeeComponents(BridgeLockFeeComponents {
            base_fee: 123,
            computed_cost_multiplier: 1,
        });

        state.put_bridge_lock_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_lock_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_unlock_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components = FeeComponents::BridgeUnlockFeeComponents(BridgeUnlockFeeComponents {
            base_fee: 123,
            computed_cost_multiplier: 1,
        });

        state.put_bridge_unlock_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_unlock_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }

    #[tokio::test]
    async fn bridge_sudo_change_fees_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let fee_components =
            FeeComponents::BridgeSudoChangeFeeComponents(BridgeSudoChangeFeeComponents {
                base_fee: 123,
                computed_cost_multiplier: 1,
            });

        state.put_bridge_sudo_change_fees(fee_components).unwrap();
        let retrieved_fee = state.get_bridge_sudo_change_fees().await.unwrap();
        assert_eq!(retrieved_fee, fee_components);
    }
}
