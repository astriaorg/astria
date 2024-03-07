use anyhow::{
    bail,
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    asset,
    Address,
    ADDRESS_LEN,
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
use ibc_types::core::channel::ChannelId;
use tracing::{
    debug,
    instrument,
};

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Balance(u128);

/// Newtype wrapper to read and write an address from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoAddress([u8; ADDRESS_LEN]);

const IBC_SUDO_STORAGE_KEY: &str = "ibcsudo";

fn channel_balance_storage_key(channel: &ChannelId, asset: asset::Id) -> String {
    format!(
        "ibc-data/{channel}/balance/{}",
        asset.encode_hex::<String>()
    )
}

fn ibc_relayer_key(address: &Address) -> String {
    format!("ibc-relayer/{}", address.encode_hex::<String>())
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
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

    #[instrument(skip(self))]
    async fn get_ibc_sudo_address(&self) -> Result<Address> {
        let Some(bytes) = self
            .get_raw(IBC_SUDO_STORAGE_KEY)
            .await
            .context("failed reading raw ibc sudo key from state")?
        else {
            // ibc sudo key must be set
            bail!("ibc sudo key not found");
        };
        let SudoAddress(address) =
            SudoAddress::try_from_slice(&bytes).context("invalid ibc sudo key bytes")?;
        Ok(Address(address))
    }

    #[instrument(skip(self))]
    async fn is_ibc_relayer(&self, address: &Address) -> Result<bool> {
        Ok(self
            .get_raw(&ibc_relayer_key(address))
            .await
            .context("failed to read ibc relayer key from state")?
            .is_some())
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_ibc_channel_balance(
        &mut self,
        channel: &ChannelId,
        asset: asset::Id,
        balance: u128,
    ) -> Result<()> {
        let bytes = borsh::to_vec(&Balance(balance)).context("failed to serialize balance")?;
        self.put_raw(channel_balance_storage_key(channel, asset), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_ibc_sudo_address(&mut self, address: Address) -> Result<()> {
        self.put_raw(
            IBC_SUDO_STORAGE_KEY.to_string(),
            borsh::to_vec(&SudoAddress(address.0))
                .context("failed to convert sudo address to vec")?,
        );
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_ibc_relayer_address(&mut self, address: &Address) {
        self.put_raw(ibc_relayer_key(address), vec![]);
    }

    #[instrument(skip(self))]
    fn delete_ibc_relayer_address(&mut self, address: &Address) {
        self.delete(ibc_relayer_key(address));
    }
}

impl<T: StateWrite> StateWriteExt for T {}
