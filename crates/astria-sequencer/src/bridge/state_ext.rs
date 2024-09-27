use std::collections::HashMap;

use astria_core::{
    generated::sequencerblock::v1alpha1::Deposit as RawDeposit,
    primitive::v1::{
        asset,
        Address,
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
        format_err,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use prost::Message as _;
use tracing::{
    debug,
    instrument,
};

use crate::{
    accounts::AddressBytes,
    address,
};

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Balance(u128);

/// Newtype wrapper to read and write a u32 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Nonce(u32);

/// Newtype wrapper to read and write a Vec<[u8; 32]> from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct AssetId([u8; 32]);

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Fee(u128);

/// A wrapper to support storing a `Vec<Deposit>`.
///
/// We don't currently have Borsh-encoding for `Deposit` and we also don't have a standalone
/// protobuf type representing a collection of `Deposit`s.
///
/// This will be replaced (very soon hopefully) by a proper storage type able to be wholly Borsh-
/// encoded. Until then, we'll protobuf-encode the individual deposits and this is a collection of
/// those encoded values.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Deposits(Vec<Vec<u8>>);

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
    async fn is_a_bridge_account<T: AddressBytes>(&self, address: T) -> Result<bool> {
        let maybe_id = self.get_bridge_account_rollup_id(address).await?;
        Ok(maybe_id.is_some())
    }

    // allow: false positive due to proc macro; fixed with rust/clippy 1.81
    #[allow(clippy::blocks_in_conditions)]
    #[instrument(skip_all, fields(address = %address.display_address()), err)]
    async fn get_bridge_account_rollup_id<T: AddressBytes>(
        &self,
        address: T,
    ) -> Result<Option<RollupId>> {
        let Some(rollup_id_bytes) = self
            .get_raw(&rollup_id_storage_key(&address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw account rollup ID from state")?
        else {
            debug!("account rollup ID not found, returning None");
            return Ok(None);
        };

        let rollup_id =
            RollupId::try_from_slice(&rollup_id_bytes).wrap_err("invalid rollup ID bytes")?;
        Ok(Some(rollup_id))
    }

    // allow: false positive due to proc macro; fixed with rust/clippy 1.81
    #[allow(clippy::blocks_in_conditions)]
    #[instrument(skip_all, fields(address = %address.display_address()), err)]
    async fn get_bridge_account_ibc_asset<T: AddressBytes>(
        &self,
        address: T,
    ) -> Result<asset::IbcPrefixed> {
        let bytes = self
            .get_raw(&asset_id_storage_key(&address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw asset ID from state")?
            .ok_or_eyre("asset ID not found")?;
        let id = borsh::from_slice::<AssetId>(&bytes)
            .wrap_err("failed to reconstruct asset ID from storage")?;
        Ok(asset::IbcPrefixed::new(id.0))
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_sudo_address<T: AddressBytes>(
        &self,
        bridge_address: T,
    ) -> Result<Option<[u8; ADDRESS_LEN]>> {
        let Some(sudo_address_bytes) = self
            .get_raw(&bridge_account_sudo_address_storage_key(&bridge_address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge account sudo address from state")?
        else {
            debug!("bridge account sudo address not found, returning None");
            return Ok(None);
        };
        let sudo_address = sudo_address_bytes.try_into().map_err(|bytes: Vec<_>| {
            format_err!(
                "failed to convert address `{}` bytes read from state to fixed length address",
                bytes.len()
            )
        })?;
        Ok(Some(sudo_address))
    }

    #[instrument(skip_all)]
    async fn get_bridge_account_withdrawer_address<T: AddressBytes>(
        &self,
        bridge_address: T,
    ) -> Result<Option<[u8; ADDRESS_LEN]>> {
        let Some(withdrawer_address_bytes) = self
            .get_raw(&bridge_account_withdrawer_address_storage_key(
                &bridge_address,
            ))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge account withdrawer address from state")?
        else {
            debug!("bridge account withdrawer address not found, returning None");
            return Ok(None);
        };
        let addr = withdrawer_address_bytes
            .try_into()
            .map_err(|bytes: Vec<_>| {
                astria_eyre::eyre::Error::msg(format!(
                    "failed converting `{}` bytes retrieved from storage to fixed address length",
                    bytes.len()
                ))
            })?;
        Ok(Some(addr))
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

        let pb_deposits = borsh::from_slice::<Deposits>(&bytes)
            .wrap_err("failed to reconstruct protobuf deposits from storage")?;

        let mut deposits = Vec::with_capacity(pb_deposits.0.len());
        for pb_deposit in pb_deposits.0 {
            let raw = RawDeposit::decode(pb_deposit.as_ref()).wrap_err("invalid deposit bytes")?;
            let deposit = Deposit::try_from_raw(raw).wrap_err("invalid deposit raw proto")?;
            deposits.push(deposit);
        }
        Ok(deposits)
    }

    #[instrument(skip_all)]
    async fn get_init_bridge_account_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw init bridge account base fee from state")?
            .ok_or_eyre("init bridge account base fee not found")?;
        let Fee(fee) = Fee::try_from_slice(&bytes).wrap_err("invalid fee bytes")?;
        Ok(fee)
    }

    #[instrument(skip_all)]
    async fn get_bridge_lock_byte_cost_multiplier(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge lock byte cost multiplier from state")?
            .ok_or_eyre("bridge lock byte cost multiplier not found")?;
        let Fee(fee) = Fee::try_from_slice(&bytes).wrap_err("invalid fee bytes")?;
        Ok(fee)
    }

    #[instrument(skip_all)]
    async fn get_bridge_sudo_change_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw bridge sudo change fee from state")?
            .ok_or_eyre("bridge sudo change fee not found")?;
        let Fee(fee) = Fee::try_from_slice(&bytes).wrap_err("invalid fee bytes")?;
        Ok(fee)
    }

    #[instrument(skip_all)]
    async fn get_last_transaction_id_for_bridge_account(
        &self,
        address: &Address,
    ) -> Result<Option<TransactionId>> {
        let Some(tx_hash_bytes) = self
            .nonverifiable_get_raw(&last_transaction_id_for_bridge_account_storage_key(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw last transaction hash for bridge account from state")?
        else {
            return Ok(None);
        };

        let tx_hash: [u8; 32] = tx_hash_bytes
            .try_into()
            .expect("all transaction hashes stored should be 32 bytes; this is a bug");

        Ok(Some(TransactionId::new(tx_hash)))
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_bridge_account_rollup_id<T: AddressBytes>(&mut self, address: T, rollup_id: &RollupId) {
        self.put_raw(rollup_id_storage_key(&address), rollup_id.to_vec());
    }

    #[instrument(skip_all)]
    fn put_bridge_account_ibc_asset<TAddress, TAsset>(
        &mut self,
        address: TAddress,
        asset: TAsset,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display,
    {
        let ibc = asset.into();
        self.put_raw(
            asset_id_storage_key(&address),
            borsh::to_vec(&AssetId(ibc.get())).wrap_err("failed to serialize asset IDs")?,
        );
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_bridge_account_sudo_address<TBridgeAddress, TSudoAddress>(
        &mut self,
        bridge_address: TBridgeAddress,
        sudo_address: TSudoAddress,
    ) where
        TBridgeAddress: AddressBytes,
        TSudoAddress: AddressBytes,
    {
        self.put_raw(
            bridge_account_sudo_address_storage_key(&bridge_address),
            sudo_address.address_bytes().to_vec(),
        );
    }

    #[instrument(skip_all)]
    fn put_bridge_account_withdrawer_address<TBridgeAddress, TWithdrawerAddress>(
        &mut self,
        bridge_address: TBridgeAddress,
        withdrawer_address: TWithdrawerAddress,
    ) where
        TBridgeAddress: AddressBytes,
        TWithdrawerAddress: AddressBytes,
    {
        self.put_raw(
            bridge_account_withdrawer_address_storage_key(&bridge_address),
            withdrawer_address.address_bytes().to_vec(),
        );
    }

    #[instrument(skip_all)]
    async fn check_and_set_withdrawal_event_block_for_bridge_account<T: AddressBytes>(
        &mut self,
        address: T,
        withdrawal_event_id: &str,
        block_num: u64,
    ) -> Result<()> {
        let key = bridge_account_withdrawal_event_storage_key(&address, withdrawal_event_id);

        // Check if the withdrawal ID has already been used, if so return an error.
        let bytes = self
            .get_raw(&key)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw withdrawal event from state")?;
        if let Some(bytes) = bytes {
            let existing_block_num = u64::from_be_bytes(
                bytes
                    .try_into()
                    .expect("all block numbers stored should be 8 bytes; this is a bug"),
            );

            bail!(
                "withdrawal event ID {withdrawal_event_id} used by block number \
                 {existing_block_num}"
            );
        }

        self.put_raw(key, block_num.to_be_bytes().to_vec());
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

    #[instrument(skip_all)]
    fn put_deposits(
        &mut self,
        block_hash: &[u8; 32],
        all_deposits: HashMap<RollupId, Vec<Deposit>>,
    ) -> Result<()> {
        for (rollup_id, deposits) in all_deposits {
            let key = deposit_storage_key(block_hash, &rollup_id);
            let serialized_deposits = deposits
                .into_iter()
                .map(|deposit| deposit.into_raw().encode_to_vec())
                .collect();
            let value = borsh::to_vec(&Deposits(serialized_deposits))
                .wrap_err("failed to serialize deposits")?;
            self.nonverifiable_put_raw(key, value);
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_init_bridge_account_base_fee(&mut self, fee: u128) {
        self.put_raw(
            INIT_BRIDGE_ACCOUNT_BASE_FEE_STORAGE_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).expect("failed to serialize fee"),
        );
    }

    #[instrument(skip_all)]
    fn put_bridge_lock_byte_cost_multiplier(&mut self, fee: u128) {
        self.put_raw(
            BRIDGE_LOCK_BYTE_COST_MULTIPLIER_STORAGE_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).expect("failed to serialize fee"),
        );
    }

    #[instrument(skip_all)]
    fn put_bridge_sudo_change_base_fee(&mut self, fee: u128) {
        self.put_raw(
            BRIDGE_SUDO_CHANGE_FEE_STORAGE_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).expect("failed to serialize fee"),
        );
    }

    #[instrument(skip_all)]
    fn put_last_transaction_id_for_bridge_account<T: AddressBytes>(
        &mut self,
        address: T,
        tx_id: &TransactionId,
    ) {
        self.nonverifiable_put_raw(
            last_transaction_id_for_bridge_account_storage_key(&address),
            tx_id.get().to_vec(),
        );
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
            state.get_bridge_account_rollup_id(address).await.expect(
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
        state.put_bridge_account_rollup_id(address, &rollup_id);
        assert_eq!(
            state
                .get_bridge_account_rollup_id(address)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id,
            "stored rollup id for bridge not what was expected"
        );

        // can rewrite with new value
        rollup_id = RollupId::new([2u8; 32]);
        state.put_bridge_account_rollup_id(address, &rollup_id);
        assert_eq!(
            state
                .get_bridge_account_rollup_id(address)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id,
            "stored rollup id for bridge not what was expected"
        );

        // can write additional account and both valid
        let rollup_id_1 = RollupId::new([2u8; 32]);
        let address_1 = astria_address(&[41u8; 20]);
        state.put_bridge_account_rollup_id(address_1, &rollup_id_1);
        assert_eq!(
            state
                .get_bridge_account_rollup_id(address_1)
                .await
                .expect("a rollup ID was written and must exist inside the database")
                .expect("expecting return value"),
            rollup_id_1,
            "additional stored rollup id for bridge not what was expected"
        );

        assert_eq!(
            state
                .get_bridge_account_rollup_id(address)
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
            .get_bridge_account_ibc_asset(address)
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
            .put_bridge_account_ibc_asset(address, &asset)
            .expect("storing bridge account asset should not fail");
        let mut result = state
            .get_bridge_account_ibc_asset(address)
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
            .put_bridge_account_ibc_asset(address, &asset)
            .expect("storing bridge account assets should not fail");
        result = state
            .get_bridge_account_ibc_asset(address)
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
            .put_bridge_account_ibc_asset(address_1, &asset_1)
            .expect("storing bridge account assets should not fail");
        assert_eq!(
            state
                .get_bridge_account_ibc_asset(address_1)
                .await
                .expect("bridge asset id was written and must exist inside the database"),
            asset_1.into(),
            "second bridge account asset not what was expected"
        );
        result = state
            .get_bridge_account_ibc_asset(address)
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
    #[allow(clippy::too_many_lines)] // allow: it's a test
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
