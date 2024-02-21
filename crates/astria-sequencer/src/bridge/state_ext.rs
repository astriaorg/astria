use anyhow::{
    Context,
    Result,
};
use astria_core::{
    generated::sequencer::v1alpha1::Deposit as RawDeposit,
    sequencer::v1alpha1::{
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

const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc";
const DEPOSIT_PREFIX: &str = "deposit";

fn storage_key(address: &str) -> String {
    format!("{BRIDGE_ACCOUNT_PREFIX}/{address}")
}

fn rollup_id_storage_key(address: Address) -> String {
    format!("{}/rollupid", storage_key(&address.encode_hex::<String>()))
}

fn deposit_storage_key_prefix(rollup_id: RollupId) -> String {
    format!("{DEPOSIT_PREFIX}/{}/", rollup_id.encode_hex::<String>())
}

fn deposit_storage_key(rollup_id: RollupId, nonce: u32) -> Vec<u8> {
    format!("{}{}", deposit_storage_key_prefix(rollup_id), nonce).into()
}

fn deposit_nonce_storage_key(rollup_id: RollupId) -> Vec<u8> {
    format!(
        "{DEPOSIT_PREFIX}/{}/nonce",
        rollup_id.encode_hex::<String>()
    )
    .into()
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_bridge_account_rollup_id(&self, address: Address) -> Result<Option<RollupId>> {
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
    async fn get_deposit_nonce(&self, rollup_id: RollupId) -> Result<u32> {
        let bytes = self
            .nonverifiable_get_raw(&deposit_nonce_storage_key(rollup_id))
            .await
            .context("failed reading raw deposit nonce from state")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(0);
        };

        let Nonce(nonce) = Nonce::try_from_slice(&bytes).context("invalid nonce bytes")?;
        Ok(nonce)
    }

    #[instrument(skip(self))]
    async fn get_deposit_rollup_ids(&self) -> Result<Vec<RollupId>> {
        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(DEPOSIT_PREFIX.as_bytes()));
        let mut rollup_ids = Vec::new();
        while let Some(Ok((_, value))) = stream.next().await {
            let rollup_id = RollupId::try_from_slice(&value).context("invalid rollup ID bytes")?;
            rollup_ids.push(rollup_id);
        }
        Ok(rollup_ids)
    }

    #[instrument(skip(self))]
    async fn get_deposit_events(&self, rollup_id: RollupId) -> Result<Vec<Deposit>> {
        let mut stream = std::pin::pin!(
            self.nonverifiable_prefix_raw(deposit_storage_key_prefix(rollup_id).as_bytes())
        );
        let mut deposits = Vec::new();
        while let Some(Ok((_, value))) = stream.next().await {
            let raw = RawDeposit::decode(value.as_ref()).context("invalid deposit bytes")?;
            let deposit = Deposit::try_from_raw(raw).context("invalid deposit bytes")?;
            deposits.push(deposit);
        }
        Ok(deposits)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_bridge_account_rollup_id(&mut self, address: Address, rollup_id: RollupId) {
        self.put_raw(rollup_id_storage_key(address), rollup_id.to_vec());
    }

    // the deposit "nonce" for a given rollup ID during a given block.
    // this is only used to generate storage keys for each of the deposits within a block,
    // and is reset to 0 at the beginning of each block.
    #[instrument(skip(self))]
    fn put_deposit_nonce(&mut self, rollup_id: RollupId, nonce: u32) {
        self.nonverifiable_put_raw(
            deposit_nonce_storage_key(rollup_id),
            nonce.to_be_bytes().to_vec(),
        );
    }

    #[instrument(skip(self))]
    async fn put_deposit_event(&mut self, deposit: Deposit) {
        let nonce = self.get_deposit_nonce(deposit.rollup_id).await.unwrap_or(0);
        self.put_deposit_nonce(deposit.rollup_id, nonce + 1);

        let key = deposit_storage_key(deposit.rollup_id, nonce);
        self.nonverifiable_put_raw(key, deposit.into_raw().encode_to_vec());
    }

    // clears the deposit nonce and all deposits for for a given rollup ID.
    #[instrument(skip(self))]
    async fn clear_deposit_info(&mut self, rollup_id: RollupId) {
        self.nonverifiable_delete(deposit_nonce_storage_key(rollup_id));
        let mut stream = std::pin::pin!(self.nonverifiable_prefix_raw(
            format!("{}/deposit/", rollup_id.encode_hex::<String>()).as_bytes()
        ));
        while let Some(Ok((key, _))) = stream.next().await {
            self.nonverifiable_delete(key);
        }
    }
}

impl<T: StateWrite> StateWriteExt for T {}
