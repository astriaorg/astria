use anyhow::{
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    Address,
    RollupId,
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
use hex::ToHex as _;
use tracing::{
    debug,
    instrument,
};

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Balance(u128);

const BRIDGE_ACCOUNT_PREFIX: &str = "bridgeacc";

fn storage_key(address: &str) -> String {
    format!("{BRIDGE_ACCOUNT_PREFIX}/{address}")
}

fn rollup_id_storage_key(address: Address) -> String {
    format!("{}/rollupid", storage_key(&address.encode_hex::<String>()))
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
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_bridge_account_rollup_id(&mut self, address: Address, rollup_id: RollupId) {
        self.put_raw(rollup_id_storage_key(address), rollup_id.to_vec());
    }
}

impl<T: StateWrite> StateWriteExt for T {}
