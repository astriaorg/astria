use std::collections::{
    HashMap,
    HashSet,
};

use anyhow::{
    anyhow,
    Context,
    Result,
};
use astria_core::{
    generated::sequencerblock::v1alpha1::Deposit as RawDeposit,
    primitive::v1::{
        asset,
        Address,
        RollupId,
    },
    sequencerblock::v1alpha1::block::Deposit,
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
use futures::StreamExt as _;
use hex::ToHex as _;
use prost::Message as _;
use tracing::{
    debug,
    instrument,
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

impl From<&asset::Id> for AssetId {
    fn from(id: &asset::Id) -> Self {
        Self(id.get())
    }
}

const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc";
const DEPOSIT_PREFIX: &str = "deposit";

fn storage_key(address: &str) -> String {
    format!("{BRIDGE_ACCOUNT_PREFIX}/{address}")
}

fn rollup_id_storage_key(address: &Address) -> String {
    format!("{}/rollupid", storage_key(&address.encode_hex::<String>()))
}

fn asset_id_storage_key(address: &Address) -> String {
    format!("{}/assetid", storage_key(&address.encode_hex::<String>()))
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

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_bridge_account_rollup_id(&self, address: &Address) -> Result<Option<RollupId>> {
        let Some(rollup_id_bytes) = self
            .get_raw(&rollup_id_storage_key(address))
            .await
            .context("failed reading raw account rollup ID from state")?
        else {
            debug!("account rollup ID not found, returning None");
            return Ok(None);
        };

        let rollup_id =
            RollupId::try_from_slice(&rollup_id_bytes).context("invalid rollup ID bytes")?;
        Ok(Some(rollup_id))
    }

    #[instrument(skip(self))]
    async fn get_bridge_account_asset_ids(&self, address: &Address) -> Result<asset::Id> {
        let bytes = self
            .get_raw(&asset_id_storage_key(address))
            .await
            .context("failed reading raw asset IDs from state")?
            .ok_or_else(|| anyhow!("asset IDs not found"))?;
        let asset_id = asset::Id::try_from_slice(&bytes).context("invalid asset IDs bytes")?;
        Ok(asset_id)
    }

    #[instrument(skip(self))]
    async fn get_deposit_nonce(&self, rollup_id: &RollupId) -> Result<u32> {
        let bytes = self
            .nonverifiable_get_raw(&deposit_nonce_storage_key(rollup_id))
            .await
            .context("failed reading raw deposit nonce from state")?;
        let Some(bytes) = bytes else {
            // no deposits for this rollup id yet; return 0
            return Ok(0);
        };

        let Nonce(nonce) =
            Nonce(u32::from_be_bytes(bytes.try_into().expect(
                "all deposit nonces stored should be 4 bytes; this is a bug",
            )));
        Ok(nonce)
    }

    #[instrument(skip(self))]
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

    #[instrument(skip(self))]
    async fn get_deposit_events(&self, rollup_id: &RollupId) -> Result<Vec<Deposit>> {
        let mut stream = std::pin::pin!(
            self.nonverifiable_prefix_raw(deposit_storage_key_prefix(rollup_id).as_bytes())
        );
        let mut deposits = Vec::new();
        while let Some(Ok((_, value))) = stream.next().await {
            let raw = RawDeposit::decode(value.as_ref()).context("invalid deposit bytes")?;
            let deposit = Deposit::try_from_raw(raw).context("invalid deposit raw proto")?;
            deposits.push(deposit);
        }
        Ok(deposits)
    }

    #[instrument(skip(self))]
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
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_bridge_account_rollup_id(&mut self, address: &Address, rollup_id: &RollupId) {
        self.put_raw(rollup_id_storage_key(address), rollup_id.to_vec());
    }

    #[instrument(skip(self))]
    fn put_bridge_account_asset_id(
        &mut self,
        address: &Address,
        asset_id: &asset::Id,
    ) -> Result<()> {
        self.put_raw(
            asset_id_storage_key(address),
            borsh::to_vec(&AssetId::from(asset_id)).context("failed to serialize asset IDs")?,
        );
        Ok(())
    }

    // the deposit "nonce" for a given rollup ID during a given block.
    // this is only used to generate storage keys for each of the deposits within a block,
    // and is reset to 0 at the beginning of each block.
    #[instrument(skip(self))]
    fn put_deposit_nonce(&mut self, rollup_id: &RollupId, nonce: u32) {
        self.nonverifiable_put_raw(
            deposit_nonce_storage_key(rollup_id),
            nonce.to_be_bytes().to_vec(),
        );
    }

    #[instrument(skip(self))]
    async fn put_deposit_event(&mut self, deposit: Deposit) -> Result<()> {
        let nonce = self.get_deposit_nonce(deposit.rollup_id()).await?;
        self.put_deposit_nonce(deposit.rollup_id(), nonce + 1);

        let key = deposit_storage_key(deposit.rollup_id(), nonce);
        self.nonverifiable_put_raw(key, deposit.into_raw().encode_to_vec());
        Ok(())
    }

    // clears the deposit nonce and all deposits for for a given rollup ID.
    #[instrument(skip(self))]
    async fn clear_deposit_info(&mut self, rollup_id: &RollupId) {
        self.nonverifiable_delete(deposit_nonce_storage_key(rollup_id));
        let mut stream = std::pin::pin!(
            self.nonverifiable_prefix_raw(deposit_storage_key_prefix(rollup_id).as_bytes())
        );
        while let Some(Ok((key, _))) = stream.next().await {
            self.nonverifiable_delete(key);
        }
    }

    #[instrument(skip(self))]
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
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset::Id,
            Address,
            RollupId,
        },
        sequencerblock::v1alpha1::block::Deposit,
    };
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn get_bridge_account_rollup_id_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let address = Address::try_from_slice(&[42u8; 20]).unwrap();

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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();

        // can write new
        state.put_bridge_account_rollup_id(&address, &rollup_id);
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
        state.put_bridge_account_rollup_id(&address, &rollup_id);
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
        let address_1 = Address::try_from_slice(&[41u8; 20]).unwrap();
        state.put_bridge_account_rollup_id(&address_1, &rollup_id_1);
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
    async fn get_bridge_account_asset_ids_none_should_fail() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        state
            .get_bridge_account_asset_ids(&address)
            .await
            .expect_err("call to get bridge account asset ids should fail if no assets");
    }

    #[tokio::test]
    async fn put_bridge_account_asset_ids() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let mut asset = Id::from_denom("asset_0");

        // can write
        state
            .put_bridge_account_asset_id(&address, &asset)
            .expect("storing bridge account asset should not fail");
        let mut result = state
            .get_bridge_account_asset_ids(&address)
            .await
            .expect("bridge asset id wwas written and must exist inside the database");
        assert_eq!(
            result, asset,
            "returned bridge account asset id did not match expected"
        );

        // can update
        asset = Id::from_denom("asset_2");
        state
            .put_bridge_account_asset_id(&address, &asset)
            .expect("storing bridge account assets should not fail");
        result = state
            .get_bridge_account_asset_ids(&address)
            .await
            .expect("bridge asset id was written and must exist inside the database");
        assert_eq!(
            result, asset,
            "returned bridge account asset id did not match expected"
        );

        // writing to other account also ok
        let address_1 = Address::try_from_slice(&[41u8; 20]).unwrap();
        let asset_1 = Id::from_denom("asset_0");
        state
            .put_bridge_account_asset_id(&address_1, &asset_1)
            .expect("storing bridge account assets should not fail");
        assert_eq!(
            state
                .get_bridge_account_asset_ids(&address_1)
                .await
                .expect("bridge asset id was written and must exist inside the database"),
            asset_1,
            "second bridge account asset not what was expected"
        );
        result = state
            .get_bridge_account_asset_ids(&address)
            .await
            .expect("original bridge asset id was written and must exist inside the database");
        assert_eq!(
            result, asset,
            "original bridge account asset id did not match expected after new bridge account added"
        );
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
        state.put_deposit_nonce(&rollup_id, nonce);
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
        state.put_deposit_nonce(&rollup_id, nonce);
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
        state.put_deposit_nonce(&rollup_id_1, nonce_1);
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
    async fn get_deposit_events() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let rollup_id = RollupId::new([1u8; 32]);
        let bridge_address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let mut amount = 10u128;
        let asset = Id::from_denom("asset_0");
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset,
            destination_chain_address.to_string(),
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
            asset,
            destination_chain_address.to_string(),
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
        let bridge_address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let amount = 10u128;
        let asset = Id::from_denom("asset_0");
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id_0,
            amount,
            asset,
            destination_chain_address.to_string(),
        );

        // write same rollup id twice
        state
            .put_deposit_event(deposit.clone())
            .await
            .expect("writing deposit events should be ok");

        // writing to same rollup id does not create duplicates
        state
            .put_deposit_event(deposit.clone())
            .await
            .expect("writing deposit events should be ok");

        // writing additional different rollup id
        let rollup_id_1 = RollupId::new([2u8; 32]);
        deposit = Deposit::new(
            bridge_address,
            rollup_id_1,
            amount,
            asset,
            destination_chain_address.to_string(),
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
        let bridge_address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let amount = 10u128;
        let asset = Id::from_denom("asset_0");
        let destination_chain_address = "0xdeadbeef";
        let deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset,
            destination_chain_address.to_string(),
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
        let bridge_address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let amount = 10u128;
        let asset = Id::from_denom("asset_0");
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset,
            destination_chain_address.to_string(),
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
            asset,
            destination_chain_address.to_string(),
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
        let bridge_address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let amount = 10u128;
        let asset = Id::from_denom("asset_0");
        let destination_chain_address = "0xdeadbeef";
        let mut deposit = Deposit::new(
            bridge_address,
            rollup_id,
            amount,
            asset,
            destination_chain_address.to_string(),
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
            asset,
            destination_chain_address.to_string(),
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
}
