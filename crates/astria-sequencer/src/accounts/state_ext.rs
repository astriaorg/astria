use anyhow::{
    Context,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    account::AssetBalance,
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
use futures::StreamExt;
use hex::ToHex as _;
use ibc_types::core::channel::ChannelId;
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

/// Newtype wrapper to read and write an address from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct SudoAddress([u8; ADDRESS_LEN]);

const ACCOUNTS_PREFIX: &str = "accounts";

const IBC_SUDO_STORAGE_KEY: &str = "ibcsudo";

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

fn ibc_relayer_key(address: &Address) -> String {
    format!("ibc-relayer/{}", address.encode_hex::<String>())
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_account_balances(&self, address: Address) -> Result<Vec<AssetBalance>> {
        use crate::asset::state_ext::StateReadExt as _;

        let prefix = format!("{}/balance/", storage_key(&address.encode_hex::<String>()));
        let mut balances: Vec<AssetBalance> = Vec::new();

        let mut stream = std::pin::pin!(self.prefix_keys(&prefix));
        while let Some(Ok(key)) = stream.next().await {
            let Some(value) = self
                .get_raw(&key)
                .await
                .context("failed reading raw account balance from state")?
            else {
                // we shouldn't receive a key in the stream with no value,
                // so this shouldn't happen
                continue;
            };

            let asset_id_str = key
                .strip_prefix(&prefix)
                .context("failed to strip prefix from account balance key")?;
            let asset_id_bytes = hex::decode(asset_id_str).context("invalid asset id bytes")?;

            let asset_id = asset::Id::try_from_slice(&asset_id_bytes)
                .context("failed to parse asset id from account balance key")?;
            let Balance(balance) =
                Balance::try_from_slice(&value).context("invalid balance bytes")?;

            let native_asset = crate::asset::get_native_asset();
            if asset_id == native_asset.id() {
                // TODO: this is jank, just have 1 denom type.
                balances.push(AssetBalance {
                    denom: astria_core::sequencer::v1alpha1::asset::Denom::from(
                        native_asset.base_denom(),
                    ),
                    balance,
                });
                continue;
            }

            let denom = self.get_ibc_asset(asset_id).await?;
            balances.push(AssetBalance {
                denom,
                balance,
            });
        }
        Ok(balances)
    }

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

    #[instrument(skip(self))]
    async fn get_ibc_sudo_address(&self) -> Result<Address> {
        let Some(bytes) = self
            .get_raw(IBC_SUDO_STORAGE_KEY)
            .await
            .context("failed reading raw ibc sudo key from state")?
        else {
            // ibc sudo key must be set
            return Err(anyhow::anyhow!("ibc sudo key not found"));
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

    #[instrument(skip(self))]
    fn put_ibc_sudo_address(&mut self, address: Address) -> Result<()> {
        self.put_raw(
            IBC_SUDO_STORAGE_KEY.to_string(),
            SudoAddress(address.0)
                .try_to_vec()
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
