use std::collections::{
    HashMap,
    HashSet,
};

use anyhow::{
    anyhow,
    bail,
    Context,
    Result,
};
use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
    },
    sequencerblock::v1alpha1::block::Deposit,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use hex::ToHex as _;
use tracing::{
    debug,
    instrument,
};

use crate::{
    accounts::AddressBytes,
    address,
    storage::{
        self,
        StoredValue,
    },
};

const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc";
const BRIDGE_ACCOUNT_SUDO_PREFIX: &str = "bsudo";
const BRIDGE_ACCOUNT_WITHDRAWER_PREFIX: &str = "bwithdrawer";
const DEPOSIT_PREFIX: &str = "deposit";
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

fn deposit_storage_key_prefix(rollup_id: &RollupId) -> String {
    format!("{DEPOSIT_PREFIX}/{}", rollup_id.encode_hex::<String>())
}

fn deposit_storage_key(rollup_id: &RollupId, nonce: u32) -> Vec<u8> {
    format!("{}/{}", deposit_storage_key_prefix(rollup_id), nonce).into()
}

fn deposit_nonce_storage_key(rollup_id: &RollupId) -> Vec<u8> {
    format!("depositnonce/{}", rollup_id.encode_hex::<String>()).into()
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
    async fn is_a_bridge_account<T: AddressBytes>(&self, address: &T) -> anyhow::Result<bool> {
        let maybe_id = self.get_bridge_account_rollup_id(address).await?;
        Ok(maybe_id.is_some())
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_rollup_id<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<Option<RollupId>> {
        let Some(bytes) = self
            .get_raw(&rollup_id_storage_key(address))
            .await
            .context("failed reading raw account rollup ID from state")?
        else {
            debug!("account rollup ID not found, returning None");
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::RollupId::try_from(value)
                    .map(|stored_rollup_id| Some(RollupId::from(stored_rollup_id)))
            })
            .context("invalid rollup ID bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_ibc_asset<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<asset::IbcPrefixed> {
        let bytes = self
            .get_raw(&asset_id_storage_key(address))
            .await
            .context("failed reading raw bridge account asset ID from state")?
            .ok_or_else(|| anyhow!("bridge account asset ID not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::IbcPrefixedDenom::try_from(value).map(asset::IbcPrefixed::from)
            })
            .context("invalid bridge account asset ID bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_sudo_address<T: AddressBytes>(
        &self,
        bridge_address: &T,
    ) -> Result<Option<[u8; ADDRESS_LEN]>> {
        let Some(bytes) = self
            .get_raw(&bridge_account_sudo_address_storage_key(bridge_address))
            .await
            .context("failed reading raw bridge account sudo address from state")?
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
            .context("invalid bridge account sudo address bytes")
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
            .context("failed reading raw bridge account withdrawer address from state")?
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
            .context("invalid bridge account withdrawer address bytes")
    }

    #[instrument(skip_all)]
    async fn get_deposit_nonce(&self, rollup_id: &RollupId) -> Result<u32> {
        let bytes = self
            .nonverifiable_get_raw(&deposit_nonce_storage_key(rollup_id))
            .await
            .context("failed reading raw deposit nonce from state")?;
        let Some(bytes) = bytes else {
            // no deposits for this rollup id yet; return 0
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Nonce::try_from(value).map(u32::from))
            .context("invalid deposit nonce bytes")
    }

    #[instrument(skip_all)]
    async fn get_deposit_rollup_ids(&self) -> Result<HashSet<RollupId>> {
        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(DEPOSIT_PREFIX.as_bytes()));
        let mut rollup_ids = HashSet::new();
        while let Some(Ok((key, _))) = stream.next().await {
            // the deposit key is of the form "deposit/{rollup_id}/{nonce}"
            let key_str =
                String::from_utf8(key).context("failed to convert deposit key to string")?;
            let key_parts = key_str.split('/').collect::<Vec<_>>();
            if key_parts.len() != 3 {
                continue;
            }
            let rollup_id_bytes =
                hex::decode(key_parts[1]).context("invalid rollup ID hex string")?;
            let rollup_id =
                RollupId::try_from_slice(&rollup_id_bytes).context("invalid rollup ID bytes")?;
            rollup_ids.insert(rollup_id);
        }
        Ok(rollup_ids)
    }

    #[instrument(skip_all)]
    async fn get_deposit_events(&self, rollup_id: &RollupId) -> Result<Vec<Deposit>> {
        let mut stream = std::pin::pin!(
            self.nonverifiable_prefix_raw(deposit_storage_key_prefix(rollup_id).as_bytes())
        );
        let mut deposits = Vec::new();
        while let Some(Ok((_, bytes))) = stream.next().await {
            let deposit = StoredValue::deserialize(&bytes)
                .and_then(|value| storage::Deposit::try_from(value).map(Deposit::from))
                .context("invalid deposit bytes")?;
            deposits.push(deposit);
        }
        Ok(deposits)
    }

    #[instrument(skip_all)]
    async fn get_block_deposits(&self) -> Result<HashMap<RollupId, Vec<Deposit>>> {
        let deposit_rollup_ids = self
            .get_deposit_rollup_ids()
            .await
            .context("failed to get deposit rollup IDs")?;
        let mut deposit_events = HashMap::new();
        for rollup_id in deposit_rollup_ids {
            let rollup_deposit_events = self
                .get_deposit_events(&rollup_id)
                .await
                .context("failed to get deposit events")?;
            deposit_events.insert(rollup_id, rollup_deposit_events);
        }
        Ok(deposit_events)
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY)
            .await
            .context("failed reading raw init bridge account base fee from state")?
            .ok_or_else(|| anyhow!("init bridge account base fee not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY)
            .await
            .context("failed reading raw bridge lock byte cost multiplier from state")?
            .ok_or_else(|| anyhow!("bridge lock byte cost multiplier not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid bridge lock byte cost multiplier bytes")
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY)
            .await
            .context("failed reading raw bridge sudo change fee from state")?
            .ok_or_else(|| anyhow!("bridge sudo change fee not found"))?;
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid bridge sudo change fee bytes")
    }

    #[instrument(skip_all)]
    async fn get_last_transaction_id_for_bridge_account<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<Option<TransactionId>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(&last_transaction_id_for_bridge_account_storage_key(address))
            .await
            .context("failed reading raw last transaction hash for bridge account from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::TransactionHash::try_from(value).map(|stored_tx_hash_bytes| {
                    Some(TransactionId::new(<[u8; 32]>::from(stored_tx_hash_bytes)))
                })
            })
            .context("invalid bridge account transaction hash bytes")
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
        let bytes = StoredValue::RollupId((&rollup_id).into())
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
        let bytes = StoredValue::IbcPrefixedDenom((&ibc).into())
            .serialize()
            .context("failed to serialize asset ids")?;
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
        let bytes = StoredValue::AddressBytes((&sudo_address).into())
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
        let bytes = StoredValue::AddressBytes((&withdrawer_address).into())
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
            .context("failed reading raw withdrawal event from state")?;
        if let Some(bytes) = bytes {
            let existing_block_num = StoredValue::deserialize(&bytes)
                .and_then(|value| storage::BlockHeight::try_from(value).map(u64::from))
                .context("invalid withdrawal event block height bytes")?;
            bail!(
                "withdrawal event ID {withdrawal_event_id} used by block number \
                 {existing_block_num}"
            );
        }

        let bytes = StoredValue::BlockHeight(block_num.into())
            .serialize()
            .context("failed to serialize withdrawal event block height")?;
        self.put_raw(key, bytes);
        Ok(())
    }

    // the deposit "nonce" for a given rollup ID during a given block.
    // this is only used to generate storage keys for each of the deposits within a block,
    // and is reset to 0 at the beginning of each block.
    #[instrument(skip_all)]
    fn put_deposit_nonce(&mut self, rollup_id: &RollupId, nonce: u32) -> Result<()> {
        let bytes = StoredValue::Nonce(nonce.into())
            .serialize()
            .context("failed to serialize deposit nonce")?;
        self.nonverifiable_put_raw(deposit_nonce_storage_key(rollup_id), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    async fn put_deposit_event(&mut self, deposit: Deposit) -> Result<()> {
        let nonce = self.get_deposit_nonce(deposit.rollup_id()).await?;
        self.put_deposit_nonce(
            deposit.rollup_id(),
            nonce.checked_add(1).context("deposit nonce overflowed")?,
        )?;

        let key = deposit_storage_key(deposit.rollup_id(), nonce);
        let bytes = StoredValue::Deposit((&deposit).into())
            .serialize()
            .context("failed to serialize bridge deposit")?;
        self.nonverifiable_put_raw(key, bytes);
        Ok(())
    }

    /// Clears the deposit nonce and all deposits for a given rollup ID.
    #[instrument(skip_all)]
    async fn clear_deposit_info(&mut self, rollup_id: &RollupId) {
        self.nonverifiable_delete(deposit_nonce_storage_key(rollup_id));
        let mut stream = std::pin::pin!(
            self.nonverifiable_prefix_raw(deposit_storage_key_prefix(rollup_id).as_bytes())
        );
        while let Some(Ok((key, _))) = stream.next().await {
            self.nonverifiable_delete(key);
        }
    }

    #[instrument(skip_all)]
    async fn clear_block_deposits(&mut self) -> Result<()> {
        let deposit_rollup_ids = self
            .get_deposit_rollup_ids()
            .await
            .context("failed to get deposit rollup ids")?;
        for rollup_id in deposit_rollup_ids {
            self.clear_deposit_info(&rollup_id).await;
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_init_bridge_account_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::Fee(fee.into())
            .serialize()
            .context("failed to serialize bridge account base fee")?;
        self.put_raw(INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_lock_byte_cost_multiplier(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::Fee(fee.into())
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
        let bytes = StoredValue::Fee(fee.into())
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
        let bytes = StoredValue::TransactionHash(tx_id.get().into())
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
mod test {
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

    use super::{
        asset_id_storage_key,
        bridge_account_sudo_address_storage_key,
        bridge_account_withdrawer_address_storage_key,
        rollup_id_storage_key,
        StateReadExt as _,
        StateWriteExt as _,
    };
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
        state
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
    async fn get_deposit_nonce_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([2u8; 32]);

        // uninitialized ok
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("call to get deposit nonce should not fail on uninitialized rollup ids"),
            0u32,
            "uninitialized rollup id nonce should be zero"
        );
    }

    #[tokio::test]
    async fn put_deposit_nonce() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([2u8; 32]);
        let mut nonce = 1u32;

        // can write
        state.put_deposit_nonce(&rollup_id, nonce).unwrap();
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("a rollup id nonce was written and must exist inside the database"),
            nonce,
            "stored nonce did not match expected"
        );

        // can update
        nonce = 2u32;
        state.put_deposit_nonce(&rollup_id, nonce).unwrap();
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("a rollup id nonce was written and must exist inside the database"),
            nonce,
            "stored nonce did not match expected"
        );

        // writing to different account is ok
        let rollup_id_1 = RollupId::new([3u8; 32]);
        let nonce_1 = 3u32;
        state.put_deposit_nonce(&rollup_id_1, nonce_1).unwrap();
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id_1)
                .await
                .expect("a rollup id nonce was written and must exist inside the database"),
            nonce_1,
            "additional stored nonce did not match expected"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("a rollup id nonce was written and must exist inside the database"),
            nonce,
            "original stored nonce did not match expected"
        );
    }

    #[tokio::test]
    async fn get_deposit_events_empty_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([2u8; 32]);

        // no events ok
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("call for rollup id with no deposit events should not fail"),
            vec![],
            "no events were written to the database so none should be returned"
        );
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)] // allow: it's a test
    async fn get_deposit_events() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let mut amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            0,
        );

        let mut deposits = vec![deposit.clone()];

        // can write
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("deposit info was written to the database and must exist"),
            deposits,
            "stored deposits do not match what was expected"
        );
        // nonce is correct
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("calls to get nonce should not fail"),
            1u32,
            "nonce was consumed and should've been incremented"
        );

        // can write additional
        amount = 20u128;
        deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            1,
        );
        deposits.append(&mut vec![deposit.clone()]);
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");
        let mut returned_deposits = state
            .get_deposit_events(&rollup_id)
            .await
            .expect("deposit info was written to the database and must exist");
        returned_deposits.sort_by_key(Deposit::amount);
        deposits.sort_by_key(Deposit::amount);
        assert_eq!(
            returned_deposits, deposits,
            "stored deposits do not match what was expected"
        );
        // nonce is correct
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("calls to get nonce should not fail"),
            2u32,
            "nonce was consumed and should've been incremented"
        );

        // can write different rollup id and both ok
        let rollup_id_1 = RollupId::new([2u8; 32]);
        deposit = Deposit::new(
            bridge_address,
            rollup_id_1,
            amount,
            asset,
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            2,
        );
        let deposits_1 = vec![deposit.clone()];
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");
        assert_eq!(
            state
                .get_deposit_events(&rollup_id_1)
                .await
                .expect("deposit info was written to the database and must exist"),
            deposits_1,
            "stored deposits do not match what was expected"
        );
        // verify original still ok
        returned_deposits = state
            .get_deposit_events(&rollup_id)
            .await
            .expect("deposit info was written to the database and must exist");
        returned_deposits.sort_by_key(Deposit::amount);
        assert_eq!(
            returned_deposits, deposits,
            "stored deposits do not match what was expected"
        );
    }

    #[tokio::test]
    async fn get_deposit_rollup_ids() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id_0 = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id_0,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            0,
        );

        // write same rollup id twice
        state
            .put_deposit_event(deposit.clone())
            .await
            .expect("writing deposit events should be ok");

        // writing to same rollup id does not create duplicates
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");

        // writing additional different rollup id
        let rollup_id_1 = RollupId::new([2u8; 32]);
        deposit = Deposit::new(
            bridge_address,
            rollup_id_1,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            1,
        );
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");
        // ensure only two rollup ids are in system
        let rollups = state
            .get_deposit_rollup_ids()
            .await
            .expect("deposit info was written rollup ids should still be in database");
        assert_eq!(rollups.len(), 2, "only two rollup ids should exits");
        assert!(
            rollups.contains(&rollup_id_0),
            "deposit data was written for rollup and it should exist"
        );
        assert!(
            rollups.contains(&rollup_id_1),
            "deposit data was written for rollup and it should exist"
        );
    }

    #[tokio::test]
    async fn clear_deposit_info_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        // uninitialized delete ok
        state.clear_deposit_info(&rollup_id).await;
    }

    #[tokio::test]
    async fn clear_deposit_info() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";
        let deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset,
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            0,
        );

        let deposits = vec![deposit.clone()];

        // can write
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("deposit info was written to the database and must exist"),
            deposits,
            "stored deposits do not match what was expected"
        );

        // can delete
        state.clear_deposit_info(&rollup_id).await;
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("deposit should return empty when none exists"),
            vec![],
            "deposits were cleared and should return empty vector"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("calls to get nonce should not fail"),
            0u32,
            "nonce should have been deleted also"
        );
    }

    #[tokio::test]
    async fn clear_deposit_info_multiple_accounts() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            0,
        );

        // write to first
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");

        // write to second
        let rollup_id_1 = RollupId::new([2u8; 32]);
        deposit = Deposit::new(
            bridge_address,
            rollup_id_1,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            1,
        );
        let deposits_1 = vec![deposit.clone()];

        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events for rollup 2 should be ok");

        // delete first rollup's info
        state.clear_deposit_info(&rollup_id).await;
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("deposit should return empty when none exists"),
            vec![],
            "deposits were cleared and should return empty vector"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("calls to get nonce should not fail"),
            0u32,
            "nonce should have been deleted also"
        );

        // second rollup's info should be intact
        assert_eq!(
            state
                .get_deposit_events(&rollup_id_1)
                .await
                .expect("deposit should return empty when none exists"),
            deposits_1,
            "deposits were written to the database and should exist"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id_1)
                .await
                .expect("calls to get nonce should not fail"),
            1u32,
            "nonce was written to the database and should exist"
        );
    }

    #[tokio::test]
    async fn clear_block_info_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // uninitialized delete ok
        state
            .clear_block_deposits()
            .await
            .expect("calls to clear block deposit should succeed");
    }

    #[tokio::test]
    async fn clear_block_deposits() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        let bridge_address = astria_address(&[42u8; 20]);
        let amount = 10u128;
        let asset = asset_0();
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            0,
        );

        // write to first
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events should be ok");

        // write to second
        let rollup_id_1 = RollupId::new([2u8; 32]);
        deposit = Deposit::new(
            bridge_address,
            rollup_id_1,
            amount,
            asset.clone(),
            destination_chain_address.to_string(),
            TransactionId::new([0; 32]),
            1,
        );
        state
            .put_deposit_event(deposit)
            .await
            .expect("writing deposit events for rollup 2 should be ok");

        // delete all info
        state
            .clear_block_deposits()
            .await
            .expect("clearing deposits call should not fail");
        assert_eq!(
            state
                .get_deposit_events(&rollup_id)
                .await
                .expect("deposit should return empty when none exists"),
            vec![],
            "deposits were cleared and should return empty vector"
        );
        // check that all info was deleted
        assert_eq!(
            state
                .get_deposit_events(&rollup_id_1)
                .await
                .expect("deposit should return empty when none exists"),
            vec![],
            "deposits were cleared and should return empty vector"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id)
                .await
                .expect("deposit should return empty when none exists"),
            0u32,
            "nonce should have been deleted also"
        );
        assert_eq!(
            state
                .get_deposit_nonce(&rollup_id_1)
                .await
                .expect("deposit should return empty when none exists"),
            0u32,
            "nonce should have been deleted also"
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
