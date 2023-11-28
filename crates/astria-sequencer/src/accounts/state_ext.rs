use anyhow::{
    Context,
    Result,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use hex::ToHex as _;
use ibc_types::core::channel::ChannelId;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use proto::native::sequencer::v1alpha1::{
    asset,
    Address,
};
use tracing::{
    debug,
    instrument,
};

/// Newtype wrapper to read and write a u32 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Nonce(u32);

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Balance(u128);

const ACCOUNTS_PREFIX: &str = "accounts";

fn storage_key(address: &str) -> String {
    format!("{ACCOUNTS_PREFIX}/{address}")
}

fn balance_storage_key(address: Address, asset: asset::Id) -> String {
    format!(
        "{}/balance/{}",
        storage_key(&address.encode_hex::<String>()),
        asset.encode_hex::<String>()
    )
}

fn nonce_storage_key(address: Address) -> String {
    format!("{}/nonce", storage_key(&address.encode_hex::<String>()))
}

fn channel_balance_storage_key(channel: &ChannelId, asset: asset::Id) -> String {
    format!(
        "ibc-data/{channel}/balance/{}",
        asset.encode_hex::<String>()
    )
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_account_balance(&self, address: Address, asset: asset::Id) -> Result<u128> {
        let Some(bytes) = self
            .get_raw(&balance_storage_key(address, asset))
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
    async fn get_account_nonce(&self, address: Address) -> Result<u32> {
        let bytes = self
            .get_raw(&nonce_storage_key(address))
            .await
            .context("failed reading raw account nonce from state")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(0);
        };

        let Nonce(nonce) = Nonce::try_from_slice(&bytes).context("invalid nonce bytes")?;
        Ok(nonce)
    }

    #[instrument(skip(self))]
    async fn get_ibc_channel_balance(&self, channel: &ChannelId, asset: asset::Id) -> Result<u128> {
        let Some(bytes) = self
            .get_raw(&channel_balance_storage_key(channel, asset))
            .await
            .context("failed reading ibc channel balance from state")?
        else {
            debug!("ibc channel balance not found, returning 0");
            return Ok(0);
        };
        let Balance(balance) = Balance::try_from_slice(&bytes).context("invalid balance bytes")?;
        Ok(balance)
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_account_balance(
        &mut self,
        address: Address,
        asset: asset::Id,
        balance: u128,
    ) -> Result<()> {
        let bytes = Balance(balance)
            .try_to_vec()
            .context("failed to serialize balance")?;
        self.put_raw(balance_storage_key(address, asset), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_account_nonce(&mut self, address: Address, nonce: u32) -> Result<()> {
        let bytes = Nonce(nonce)
            .try_to_vec()
            .context("failed to serialize nonce")?;
        self.put_raw(nonce_storage_key(address), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_ibc_channel_balance(
        &mut self,
        channel: &ChannelId,
        asset: asset::Id,
        balance: u128,
    ) -> Result<()> {
        let bytes = Balance(balance)
            .try_to_vec()
            .context("failed to serialize balance")?;
        self.put_raw(channel_balance_storage_key(channel, asset), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
