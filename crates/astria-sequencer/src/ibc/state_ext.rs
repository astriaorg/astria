use anyhow::{
    bail,
    Context,
    Result,
};
use astria_core::primitive::v1::{
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

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Fee(u128);

const IBC_SUDO_STORAGE_KEY: &str = "ibcsudo";
const ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY: &str = "ics20withdrawalfee";

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
        Ok(Address::from(address))
    }

    #[instrument(skip(self))]
    async fn is_ibc_relayer(&self, address: &Address) -> Result<bool> {
        Ok(self
            .get_raw(&ibc_relayer_key(address))
            .await
            .context("failed to read ibc relayer key from state")?
            .is_some())
    }

    #[instrument(skip(self))]
    async fn get_ics20_withdrawal_base_fee(&self) -> Result<u128> {
        let Some(bytes) = self
            .get_raw(ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY)
            .await
            .context("failed reading ics20 withdrawal fee from state")?
        else {
            bail!("ics20 withdrawal fee not found");
        };
        let Fee(fee) = Fee::try_from_slice(&bytes).context("invalid fee bytes")?;
        Ok(fee)
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
            borsh::to_vec(&SudoAddress(address.get()))
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

    #[instrument(skip(self))]
    fn put_ics20_withdrawal_base_fee(&mut self, fee: u128) -> Result<()> {
        self.put_raw(
            ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY.to_string(),
            borsh::to_vec(&Fee(fee)).context("failed to serialize fee")?,
        );
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use astria_core::primitive::v1::{
        asset::Id,
        Address,
    };
    use cnidarium::StateDelta;
    use ibc_types::core::channel::ChannelId;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    #[tokio::test]
    async fn get_ibc_sudo_address_fails_if_not_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // should fail if not set
        state
            .get_ibc_sudo_address()
            .await
            .expect_err("sudo address should be set");
    }

    #[tokio::test]
    async fn put_ibc_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write new
        let mut address = Address::try_from_slice(&[42u8; 20]).unwrap();
        state
            .put_ibc_sudo_address(address)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state
                .get_ibc_sudo_address()
                .await
                .expect("a sudo address was written and must exist inside the database"),
            address,
            "stored sudo address was not what was expected"
        );

        // can rewrite with new value
        address = Address::try_from_slice(&[41u8; 20]).unwrap();
        state
            .put_ibc_sudo_address(address)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state
                .get_ibc_sudo_address()
                .await
                .expect("sudo address was written and must exist inside the database"),
            address,
            "updated sudo address was not what was expected"
        );
    }

    #[tokio::test]
    async fn is_ibc_relayer_ok_if_not_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // unset address returns false
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        assert!(
            !state
                .is_ibc_relayer(&address)
                .await
                .expect("calls to properly formatted addresses should not fail"),
            "inputted address should've returned false"
        );
    }

    #[tokio::test]
    async fn delete_ibc_relayer_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        state.put_ibc_relayer_address(&address);
        assert!(
            state
                .is_ibc_relayer(&address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can delete
        state.delete_ibc_relayer_address(&address);
        assert!(
            !state
                .is_ibc_relayer(&address)
                .await
                .expect("calls on unset addresses should not fail"),
            "relayer address was not deleted as was intended"
        );
    }

    #[tokio::test]
    async fn put_ibc_relayer_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        state.put_ibc_relayer_address(&address);
        assert!(
            state
                .is_ibc_relayer(&address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can write multiple
        let address_1 = Address::try_from_slice(&[41u8; 20]).unwrap();
        state.put_ibc_relayer_address(&address_1);
        assert!(
            state
                .is_ibc_relayer(&address_1)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "additional stored relayer address could not be verified"
        );
        assert!(
            state
                .is_ibc_relayer(&address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "original stored relayer address could not be verified"
        );
    }

    #[tokio::test]
    async fn get_ibc_channel_balance_unset_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let channel = ChannelId::new(0u64);
        let asset = Id::from_denom("asset");

        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            0u128,
            "unset asset and channel should return zero"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let channel = ChannelId::new(0u64);
        let asset = Id::from_denom("asset");
        let mut amount = 10u128;

        // write initial
        state
            .put_ibc_channel_balance(&channel, asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );

        // can update
        amount = 20u128;
        state
            .put_ibc_channel_balance(&channel, asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_mutliple_assets() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let channel = ChannelId::new(0u64);
        let asset_0 = Id::from_denom("asset_0");
        let asset_1 = Id::from_denom("asset_1");
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state
            .put_ibc_channel_balance(&channel, asset_0, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state
            .put_ibc_channel_balance(&channel, asset_1, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, asset_0)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, asset_1)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_mutliple_channels() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let channel_0 = ChannelId::new(0u64);
        let channel_1 = ChannelId::new(1u64);
        let asset = Id::from_denom("asset_0");
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state
            .put_ibc_channel_balance(&channel_0, asset, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state
            .put_ibc_channel_balance(&channel_1, asset, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel_0, asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel_1, asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }
}
