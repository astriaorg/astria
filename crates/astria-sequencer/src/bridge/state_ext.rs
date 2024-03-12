use std::collections::HashMap;

use anyhow::{
    anyhow,
    Context,
    Result,
};
use astria_core::{
    generated::sequencer::v1alpha1::Deposit as RawDeposit,
    sequencer::v1alpha1::{
        asset,
        block::Deposit,
        Address,
        RollupId,
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
struct AssetIds(Vec<[u8; 32]>);

impl From<&[asset::Id]> for AssetIds {
    fn from(ids: &[asset::Id]) -> Self {
        Self(ids.iter().copied().map(asset::Id::get).collect())
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

fn asset_ids_storage_key(address: &Address) -> String {
    format!("{}/assetids", storage_key(&address.encode_hex::<String>()))
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
    async fn get_bridge_account_asset_ids(&self, address: &Address) -> Result<Vec<asset::Id>> {
        let bytes = self
            .get_raw(&asset_ids_storage_key(address))
            .await
            .context("failed reading raw asset IDs from state")?
            .ok_or_else(|| anyhow!("asset IDs not found"))?;
        let asset_ids = AssetIds::try_from_slice(&bytes).context("invalid asset IDs bytes")?;
        Ok(asset_ids.0.into_iter().map(asset::Id::from).collect())
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

        let Nonce(nonce) = Nonce::try_from_slice(&bytes).context("invalid nonce bytes")?;
        Ok(nonce)
    }

    #[instrument(skip(self))]
    async fn get_deposit_rollup_ids(&self) -> Result<Vec<RollupId>> {
        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(DEPOSIT_PREFIX.as_bytes()));
        let mut rollup_ids = Vec::new();
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
            rollup_ids.push(rollup_id);
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
    fn put_bridge_account_asset_ids(
        &mut self,
        address: &Address,
        asset_ids: &[asset::Id],
    ) -> Result<()> {
        self.put_raw(
            asset_ids_storage_key(address),
            borsh::to_vec(&AssetIds::from(asset_ids)).context("failed to serialize asset IDs")?,
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
