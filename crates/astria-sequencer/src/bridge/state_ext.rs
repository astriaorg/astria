use anyhow::{
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::RollupId;
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

fn storage_key(rollup_id: &str) -> String {
    format!("{BRIDGE_ACCOUNT_PREFIX}/{rollup_id}")
}

fn balance_storage_key(rollup_id: RollupId) -> String {
    format!("{}/balance", storage_key(&rollup_id.encode_hex::<String>()))
}

fn pubkey_storage_key(rollup_id: RollupId) -> String {
    format!("{}/pubkey", storage_key(&rollup_id.encode_hex::<String>()))
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_bridge_account_balance(&self, rollup_id: RollupId) -> Result<u128> {
        let Some(bytes) = self
            .get_raw(&balance_storage_key(rollup_id))
            .await
            .context("failed reading raw account balance from state")?
        else {
            debug!("account balance not found, returning 0");
            return Ok(0);
        };

        let Balance(balance) = Balance::try_from_slice(&bytes).context("invalid balance bytes")?;
        Ok(balance)
    }

    #[instrument(skip(self))]
    async fn get_bridge_account_pubkey(
        &self,
        rollup_id: RollupId,
    ) -> Result<Option<ed25519_consensus::VerificationKey>> {
        let Some(pubkey_bytes) = self
            .get_raw(&pubkey_storage_key(rollup_id))
            .await
            .context("failed reading raw account pubkey from state")?
        else {
            debug!("account pubkey not found, returning None");
            return Ok(None);
        };

        let pubkey = ed25519_consensus::VerificationKey::try_from(pubkey_bytes.as_slice())
            .context("failed to parse account pubkey from bytes")?;
        Ok(Some(pubkey))
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_bridge_account_balance(&mut self, rollup_id: RollupId, balance: u128) -> Result<()> {
        let bytes = Balance(balance)
            .try_to_vec()
            .context("failed to serialize balance")?;
        self.put_raw(balance_storage_key(rollup_id), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_bridge_account_pubkey(
        &mut self,
        rollup_id: RollupId,
        pubkey: ed25519_consensus::VerificationKey,
    ) {
        self.put_raw(pubkey_storage_key(rollup_id), pubkey.as_bytes().to_vec());
    }
}

impl<T: StateWrite> StateWriteExt for T {}
