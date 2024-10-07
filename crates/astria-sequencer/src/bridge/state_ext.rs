use std::collections::HashMap;

use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
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
use tracing::{
    debug,
    instrument,
};

use super::storage;
use crate::{
    accounts::AddressBytes,
    address,
    storage::StoredValue,
};

const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc";
const BRIDGE_ACCOUNT_SUDO_PREFIX: &str = "bsudo";
const BRIDGE_ACCOUNT_WITHDRAWER_PREFIX: &str = "bwithdrawer";
const DEPOSITS_EPHEMERAL_KEY: &str = "deposits";
const DEPOSIT_PREFIX: &[u8] = b"deposit/";
const INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY: &str = "initbridgeaccfee";
const BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY: &str = "bridgelockmultiplier";
const BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY: &str = "bridgesudofee";

struct BridgeAccountKey<'a, T> {
    prefix: &'static str,
    address: &'a T,
}

impl<'a, T> std::fmt::Display for BridgeAccountKey<'a, T>
where
    T: AddressBytes,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.prefix)?;
        f.write_str("/")?;
        for byte in self.address.address_bytes() {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

fn rollup_id_storage_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/rollupid",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_PREFIX,
            address
        }
    )
}

fn asset_id_storage_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/assetid",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_PREFIX,
            address
        }
    )
}

fn deposit_storage_key(block_hash: &[u8; 32], rollup_id: &RollupId) -> Vec<u8> {
    [DEPOSIT_PREFIX, block_hash, rollup_id.as_ref()].concat()
}

fn bridge_account_sudo_address_storage_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_SUDO_PREFIX,
            address
        }
    )
}

fn bridge_account_withdrawer_address_storage_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_WITHDRAWER_PREFIX,
            address
        }
    )
}

fn bridge_account_withdrawal_event_storage_key<T: AddressBytes>(
    address: &T,
    withdrawal_event_id: &str,
) -> String {
    format!(
        "{}/withdrawalevent/{}",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_PREFIX,
            address
        },
        withdrawal_event_id
    )
}

fn last_transaction_id_for_bridge_account_storage_key<T: AddressBytes>(address: &T) -> Vec<u8> {
    format!(
        "{}/lasttx",
        BridgeAccountKey {
            prefix: BRIDGE_ACCOUNT_PREFIX,
            address
        }
    )
    .as_bytes()
    .to_vec()
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead + address::StateReadExt {
    #[instrument(skip_all)]
    async fn is_a_bridge_account<T: AddressBytes>(&self, address: &T) -> Result<bool> {
        let maybe_id = self.get_bridge_account_rollup_id(address).await?;
        Ok(maybe_id.is_some())
    }

    #[instrument(skip_all, fields(address = %address.display_address()), err)]
    async fn get_bridge_account_rollup_id<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<Option<RollupId>> {
        let Some(bytes) = self
            .get_raw(&rollup_id_storage_key(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw account rollup ID from state")?
        else {
            debug!("account rollup ID not found, returning None");
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::RollupId::try_from(value)
                    .map(|stored_rollup_id| Some(RollupId::from(stored_rollup_id)))
            })
            .wrap_err("invalid rollup ID bytes")
    }

    #[instrument(skip_all, fields(address = %address.display_address()), err)]
    async fn get_bridge_account_ibc_asset<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<asset::IbcPrefixed> {
        let bytes = self
            .get_raw(&asset_id_storage_key(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge account asset ID from state")?
            .ok_or_eyre("bridge account asset ID not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcPrefixedDenom::try_from(value).map(asset::IbcPrefixed::from)
            })
            .wrap_err("invalid bridge account asset ID bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_sudo_address<T: AddressBytes>(
        &self,
        bridge_address: &T,
    ) -> Result<Option<[u8; ADDRESS_LEN]>> {
        let Some(bytes) = self
            .get_raw(&bridge_account_sudo_address_storage_key(bridge_address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge account sudo address from state")?
        else {
            debug!("bridge account sudo address not found, returning None");
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::AddressBytes::try_from(value).map(|stored_address_bytes| {
                    Some(<[u8; ADDRESS_LEN]>::from(stored_address_bytes))
                })
            })
            .wrap_err("invalid bridge account sudo address bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_withdrawer_address<T: AddressBytes>(
        &self,
        bridge_address: &T,
    ) -> Result<Option<[u8; ADDRESS_LEN]>> {
        let Some(bytes) = self
            .get_raw(&bridge_account_withdrawer_address_storage_key(
                bridge_address,
            ))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge account withdrawer address from state")?
        else {
            debug!("bridge account withdrawer address not found, returning None");
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::AddressBytes::try_from(value).map(|stored_address_bytes| {
                    Some(<[u8; ADDRESS_LEN]>::from(stored_address_bytes))
                })
            })
            .wrap_err("invalid bridge account withdrawer address bytes")
    }

    #[instrument(skip_all)]
    fn get_cached_block_deposits(&self) -> HashMap<RollupId, Vec<Deposit>> {
        self.object_get(DEPOSITS_EPHEMERAL_KEY).unwrap_or_default()
    }

    #[instrument(skip_all)]
    async fn get_deposits(
        &self,
        block_hash: &[u8; 32],
        rollup_id: &RollupId,
    ) -> Result<Vec<Deposit>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(&deposit_storage_key(block_hash, rollup_id))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw deposits from state")?
        else {
            return Ok(vec![]);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Deposits::try_from(value).map(Vec::<Deposit>::from))
            .context("invalid deposits bytes")
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw init bridge account base fee from state")?
            .ok_or_eyre("init bridge account base fee not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .wrap_err("invalid fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge lock byte cost multiplier from state")?
            .ok_or_eyre("bridge lock byte cost multiplier not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .wrap_err("invalid bridge lock byte cost multiplier bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge sudo change fee from state")?
            .ok_or_eyre("bridge sudo change fee not found")?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .wrap_err("invalid bridge sudo change fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_last_transaction_id_for_bridge_account<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<Option<TransactionId>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(&last_transaction_id_for_bridge_account_storage_key(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw last transaction hash for bridge account from state")?
        else {
            return Ok(None);
        };
        let tx_id = StoredValue::deserialize(&bytes)
            .and_then(|value| storage::TransactionId::try_from(value).map(TransactionId::from))
            .wrap_err("invalid bridge account transaction hash bytes")?;
        Ok(Some(tx_id))
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_bridge_account_rollup_id<T: AddressBytes>(
        &mut self,
        address: &T,
        rollup_id: RollupId,
    ) -> Result<()> {
        let bytes = StoredValue::from(storage::RollupId::from(&rollup_id))
            .serialize()
            .context("failed to serialize bridge account rollup id")?;
        self.put_raw(rollup_id_storage_key(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_account_ibc_asset<TAddress, TAsset>(
        &mut self,
        address: &TAddress,
        asset: TAsset,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed>,
    {
        let ibc = asset.into();
        let bytes = StoredValue::from(storage::IbcPrefixedDenom::from(&ibc))
            .serialize()
            .wrap_err("failed to serialize asset ids")?;
        self.put_raw(asset_id_storage_key(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_account_sudo_address<TBridgeAddress, TSudoAddress>(
        &mut self,
        bridge_address: &TBridgeAddress,
        sudo_address: TSudoAddress,
    ) -> Result<()>
    where
        TBridgeAddress: AddressBytes,
        TSudoAddress: AddressBytes,
    {
        let bytes = StoredValue::from(storage::AddressBytes::from(&sudo_address))
            .serialize()
            .context("failed to serialize bridge account sudo address")?;
        self.put_raw(
            bridge_account_sudo_address_storage_key(bridge_address),
            bytes,
        );
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_account_withdrawer_address<TBridgeAddress, TWithdrawerAddress>(
        &mut self,
        bridge_address: &TBridgeAddress,
        withdrawer_address: TWithdrawerAddress,
    ) -> Result<()>
    where
        TBridgeAddress: AddressBytes,
        TWithdrawerAddress: AddressBytes,
    {
        let bytes = StoredValue::from(storage::AddressBytes::from(&withdrawer_address))
            .serialize()
            .context("failed to serialize bridge account sudo address")?;
        self.put_raw(
            bridge_account_withdrawer_address_storage_key(bridge_address),
            bytes,
        );
        Ok(())
    }

    #[instrument(skip_all)]
    async fn check_and_set_withdrawal_event_block_for_bridge_account<T: AddressBytes>(
        &mut self,
        address: &T,
        withdrawal_event_id: &str,
        block_num: u64,
    ) -> Result<()> {
        let key = bridge_account_withdrawal_event_storage_key(address, withdrawal_event_id);

        // Check if the withdrawal ID has already been used, if so return an error.
        let bytes = self
            .get_raw(&key)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw withdrawal event from state")?;
        if let Some(bytes) = bytes {
            let existing_block_num = StoredValue::deserialize(&bytes)
                .and_then(|value| storage::BlockHeight::try_from(value).map(u64::from))
                .context("invalid withdrawal event block height bytes")?;
            bail!(
                "withdrawal event ID {withdrawal_event_id} used by block number \
                 {existing_block_num}"
            );
        }

        let bytes = StoredValue::from(storage::BlockHeight::from(block_num))
            .serialize()
            .context("failed to serialize withdrawal event block height")?;
        self.put_raw(key, bytes);
        Ok(())
    }

    /// Push the deposit onto the end of a Vec of deposits for this rollup ID.  These are held in
    /// state's ephemeral store, pending being written to permanent storage during `finalize_block`.
    #[instrument(skip_all)]
    fn cache_deposit_event(&mut self, deposit: Deposit) {
        let mut cached_deposits = self.get_cached_block_deposits();
        cached_deposits
            .entry(deposit.rollup_id)
            .or_default()
            .push(deposit);
        self.object_put(DEPOSITS_EPHEMERAL_KEY, cached_deposits);
    }

    #[instrument(skip_all, err)]
    fn put_deposits(
        &mut self,
        block_hash: &[u8; 32],
        all_deposits: HashMap<RollupId, Vec<Deposit>>,
    ) -> Result<()> {
        for (rollup_id, deposits) in all_deposits {
            let key = deposit_storage_key(block_hash, &rollup_id);
            let bytes = StoredValue::from(storage::Deposits::from(deposits.iter()))
                .serialize()
                .context("failed to serialize bridge deposit")?;
            self.nonverifiable_put_raw(key, bytes);
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_init_bridge_account_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::from(storage::Fee::from(fee))
            .serialize()
            .context("failed to serialize bridge account base fee")?;
        self.put_raw(INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_lock_byte_cost_multiplier(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::from(storage::Fee::from(fee))
            .serialize()
            .context("failed to serialize bridge lock byte cost multiplier")?;
        self.put_raw(
            BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY.to_string(),
            bytes,
        );
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_sudo_change_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::from(storage::Fee::from(fee))
            .serialize()
            .context("failed to serialize bridge sudo change base fee")?;
        self.put_raw(BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_last_transaction_id_for_bridge_account<T: AddressBytes>(
        &mut self,
        address: &T,
        tx_id: TransactionId,
    ) -> Result<()> {
        let bytes = StoredValue::from(storage::TransactionId::from(&tx_id))
            .serialize()
            .context("failed to serialize transaction hash for bridge account")?;
        self.nonverifiable_put_raw(
            last_transaction_id_for_bridge_account_storage_key(address),
            bytes,
        );
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset,
            Address,
            RollupId,
            TransactionId,
        },
        sequencerblock::v1alpha1::block::Deposit,
    };
    use cnidarium::StateDelta;
    use insta::assert_snapshot;

    use super::*;
    use crate::test_utils::astria_address;

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn get_bridge_account_rollup_id_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let address = astria_address(&[42u8; 20]);

        // uninitialized ok
        assert_eq!(
            state.get_bridge_account_rollup_id(&address).await.expect(
                "call to get bridge account rollup id should not fail for uninitialized addresses"
            ),
            Option::None,
            "stored rollup id for bridge not what was expected"
        );
    }

    #[tokio::test]
    async fn put_bridge_account_rollup_id() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let mut rollup_id = RollupId::new([1u8; 32]);
        let address = astria_address(&[42u8; 20]);

        // can write new
        state
            .put_bridge_account_rollup_id(&address, rollup_id)
            .unwrap();
        assert_eq!(
            state
                .get_bridge_account_rollup_id(&address)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id,
            "stored rollup id for bridge not what was expected"
        );

        // can rewrite with new value
        rollup_id = RollupId::new([2u8; 32]);
        state
            .put_bridge_account_rollup_id(&address, rollup_id)
            .unwrap();
        assert_eq!(
            state
                .get_bridge_account_rollup_id(&address)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id,
            "stored rollup id for bridge not what was expected"
        );

        // can write additional account and both valid
        let rollup_id_1 = RollupId::new([2u8; 32]);
        let address_1 = astria_address(&[41u8; 20]);
        state
            .put_bridge_account_rollup_id(&address_1, rollup_id_1)
            .unwrap();
        assert_eq!(
            state
                .get_bridge_account_rollup_id(&address_1)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id_1,
            "additional stored rollup id for bridge not what was expected"
        );

        assert_eq!(
            state
                .get_bridge_account_rollup_id(&address)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id,
            "original stored rollup id for bridge not what was expected"
        );
    }

    #[tokio::test]
    async fn get_bridge_account_asset_id_none_should_fail() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let address = astria_address(&[42u8; 20]);
        let _ = state
            .get_bridge_account_ibc_asset(&address)
            .await
            .expect_err("call to get bridge account asset ids should fail if no assets");
    }

    #[tokio::test]
    async fn put_bridge_account_ibc_assets() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let address = astria_address(&[42u8; 20]);
        let mut asset = asset_0();

        // can write
        state
            .put_bridge_account_ibc_asset(&address, asset.clone())
            .expect("storing bridge account asset should not fail");
        let mut result = state
            .get_bridge_account_ibc_asset(&address)
            .await
            .expect("bridge asset id was written and must exist inside the database");
        assert_eq!(
            result,
            asset.to_ibc_prefixed(),
            "returned bridge account asset id did not match expected"
        );

        // can update
        asset = "asset_2".parse::<asset::Denom>().unwrap();
        state
            .put_bridge_account_ibc_asset(&address, &asset)
            .expect("storing bridge account assets should not fail");
        result = state
            .get_bridge_account_ibc_asset(&address)
            .await
            .expect("bridge asset id was written and must exist inside the database");
        assert_eq!(
            result,
            asset.to_ibc_prefixed(),
            "returned bridge account asset id did not match expected"
        );

        // writing to other account also ok
        let address_1 = astria_address(&[41u8; 20]);
        let asset_1 = asset_1();
        state
            .put_bridge_account_ibc_asset(&address_1, &asset_1)
            .expect("storing bridge account assets should not fail");
        assert_eq!(
            state
                .get_bridge_account_ibc_asset(&address_1)
                .await
                .expect("bridge asset id was written and must exist inside the database"),
            asset_1.into(),
            "second bridge account asset not what was expected"
        );
        result = state
            .get_bridge_account_ibc_asset(&address)
            .await
            .expect("original bridge asset id was written and must exist inside the database");
        assert_eq!(
            result,
            asset.to_ibc_prefixed(),
            "original bridge account asset id did not match expected after new bridge account \
             added"
        );
    }

    #[tokio::test]
    async fn bridge_account_sudo_address_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = [1; 20];
        let sudo_address = [2; 20];
        state
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();
        let retrieved_sudo_address = state
            .get_bridge_account_sudo_address(&bridge_address)
            .await
            .unwrap();
        assert_eq!(retrieved_sudo_address, Some(sudo_address));
    }

    #[tokio::test]
    async fn bridge_account_withdrawer_address_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = [1; 20];
        let withdrawer_address = [2; 20];
        state
            .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
            .unwrap();
        let retrieved_withdrawer_address = state
            .get_bridge_account_withdrawer_address(&bridge_address)
            .await
            .unwrap();
        assert_eq!(retrieved_withdrawer_address, Some(withdrawer_address));
    }

    #[tokio::test]
    async fn get_deposits_empty_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let block_hash = [32; 32];
        let rollup_id = RollupId::new([2u8; 32]);

        // no events ok
        assert_eq!(
            state
                .get_deposits(&block_hash, &rollup_id)
                .await
                .expect("call for rollup id with no deposit events should not fail"),
            vec![],
            "no events were written to the database so none should be returned"
        );
    }

    #[tokio::test]
    async fn get_deposits() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let block_hash = [32; 32];
        let rollup_id_1 = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";

        let mut deposit = Deposit {
            bridge_address,
            rollup_id: rollup_id_1,
            amount,
            asset: asset.clone(),
            destination_chain_address: destination_chain_address.to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };

        let mut all_deposits = HashMap::new();
        let mut rollup_1_deposits = vec![deposit.clone()];
        all_deposits.insert(rollup_id_1, rollup_1_deposits.clone());

        // can write
        state
            .put_deposits(&block_hash, all_deposits.clone())
            .unwrap();
        assert_eq!(
            state
                .get_deposits(&block_hash, &rollup_id_1)
                .await
                .expect("deposit info was written to the database and must exist"),
            rollup_1_deposits,
            "stored deposits do not match what was expected"
        );

        // can write additional
        deposit = Deposit {
            amount,
            source_action_index: 1,
            ..deposit
        };
        rollup_1_deposits.push(deposit.clone());
        all_deposits.insert(rollup_id_1, rollup_1_deposits.clone());
        state
            .put_deposits(&block_hash, all_deposits.clone())
            .unwrap();
        assert_eq!(
            state
                .get_deposits(&block_hash, &rollup_id_1)
                .await
                .expect("deposit info was written to the database and must exist"),
            rollup_1_deposits,
            "stored deposits do not match what was expected"
        );

        // can write different rollup id and both ok
        let rollup_id_2 = RollupId::new([2u8; 32]);
        deposit = Deposit {
            rollup_id: rollup_id_2,
            source_action_index: 2,
            ..deposit
        };
        let rollup_2_deposits = vec![deposit.clone()];
        all_deposits.insert(rollup_id_2, rollup_2_deposits.clone());
        state.put_deposits(&block_hash, all_deposits).unwrap();
        assert_eq!(
            state
                .get_deposits(&block_hash, &rollup_id_2)
                .await
                .expect("deposit info was written to the database and must exist"),
            rollup_2_deposits,
            "stored deposits do not match what was expected"
        );
        // verify original still ok
        assert_eq!(
            state
                .get_deposits(&block_hash, &rollup_id_1)
                .await
                .expect("deposit info was written to the database and must exist"),
            rollup_1_deposits,
            "stored deposits do not match what was expected"
        );
    }

    #[tokio::test]
    async fn init_bridge_account_base_fee_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_init_bridge_account_base_fee(123).unwrap();
        let retrieved_fee = state.get_init_bridge_account_base_fee().await.unwrap();
        assert_eq!(retrieved_fee, 123);
    }

    #[tokio::test]
    async fn bridge_lock_byte_cost_multiplier_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_bridge_lock_byte_cost_multiplier(123).unwrap();
        let retrieved_fee = state.get_bridge_lock_byte_cost_multiplier().await.unwrap();
        assert_eq!(retrieved_fee, 123);
    }

    #[tokio::test]
    async fn bridge_sudo_change_base_fee_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_bridge_sudo_change_base_fee(123).unwrap();
        let retrieved_fee = state.get_bridge_sudo_change_base_fee().await.unwrap();
        assert_eq!(retrieved_fee, 123);
    }

    #[tokio::test]
    async fn last_transaction_id_for_bridge_account_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let bridge_address = [1; 20];
        let tx_hash = TransactionId::new([2; 32]);
        state
            .put_last_transaction_id_for_bridge_account(&bridge_address, tx_hash)
            .unwrap();
        let retrieved_tx_hash = state
            .get_last_transaction_id_for_bridge_account(&bridge_address)
            .await
            .unwrap();
        assert_eq!(retrieved_tx_hash, Some(tx_hash));
    }

    #[test]
    fn storage_keys_have_not_changed() {
        let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap();

        assert_snapshot!(rollup_id_storage_key(&address));
        assert_snapshot!(asset_id_storage_key(&address));
        assert_snapshot!(bridge_account_sudo_address_storage_key(&address));
        assert_snapshot!(bridge_account_withdrawer_address_storage_key(&address));
    }
}
