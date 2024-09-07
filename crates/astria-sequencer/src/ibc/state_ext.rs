use anyhow::{
    bail,
    Context,
    Result,
};
use astria_core::primitive::v1::{
    asset,
    ADDRESS_LEN,
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use ibc_types::core::channel::ChannelId;
use tracing::{
    debug,
    instrument,
};

use crate::{
    accounts::AddressBytes,
    storage::{
        self,
        StoredValue,
    },
};

const IBC_SUDO_STORAGE_KEY: &str = "ibcsudo";
const ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY: &str = "ics20withdrawalfee";

struct IbcRelayerKey<'a, T>(&'a T);

impl<'a, T: AddressBytes> std::fmt::Display for IbcRelayerKey<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ibc-relayer")?;
        f.write_str("/")?;
        for byte in self.0.address_bytes() {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

fn channel_balance_storage_key<'a, TAsset>(channel: &ChannelId, asset: &'a TAsset) -> String
where
    asset::IbcPrefixed: From<&'a TAsset>,
{
    format!(
        "ibc-data/{channel}/balance/{}",
        crate::storage_keys::hunks::Asset::from(asset),
    )
}

fn ibc_relayer_key<T: AddressBytes>(address: &T) -> String {
    IbcRelayerKey(address).to_string()
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_ibc_channel_balance<'a, TAsset>(
        &self,
        channel: &ChannelId,
        asset: &'a TAsset,
    ) -> Result<u128>
    where
        TAsset: Sync,
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let Some(bytes) = self
            .get_raw(&channel_balance_storage_key(channel, asset))
            .await
            .context("failed reading ibc channel balance from state")?
        else {
            debug!("ibc channel balance not found, returning 0");
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Balance::try_from(value).map(u128::from))
            .context("invalid ibc channel balance bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_sudo_address(&self) -> Result<[u8; ADDRESS_LEN]> {
        let Some(bytes) = self
            .get_raw(IBC_SUDO_STORAGE_KEY)
            .await
            .context("failed reading raw ibc sudo address from state")?
        else {
            // ibc sudo key must be set
            bail!("ibc sudo address not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::AddressBytes::try_from(value).map(<[u8; ADDRESS_LEN]>::from))
            .context("invalid ibc sudo address bytes")
    }

    #[instrument(skip_all)]
    async fn is_ibc_relayer<T: AddressBytes>(&self, address: T) -> Result<bool> {
        Ok(self
            .get_raw(&ibc_relayer_key(&address))
            .await
            .context("failed to read ibc relayer key from state")?
            .is_some())
    }

    #[instrument(skip_all)]
    async fn get_ics20_withdrawal_base_fee(&self) -> Result<u128> {
        let Some(bytes) = self
            .get_raw(ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY)
            .await
            .context("failed reading ics20 withdrawal fee from state")?
        else {
            bail!("ics20 withdrawal fee not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Fee::try_from(value).map(u128::from))
            .context("invalid ics20 withdrawal base fee bytes")
    }
}

impl<T: StateRead> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_ibc_channel_balance<'a, TAsset>(
        &mut self,
        channel: &ChannelId,
        asset: &'a TAsset,
        balance: u128,
    ) -> Result<()>
    where
        asset::IbcPrefixed: From<&'a TAsset>,
    {
        let bytes = StoredValue::Balance(balance.into())
            .serialize()
            .context("failed to serialize ibc channel balance")?;
        self.put_raw(channel_balance_storage_key(channel, asset), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_sudo_address<T: AddressBytes>(&mut self, address: T) -> Result<()> {
        let bytes = StoredValue::AddressBytes((&address).into())
            .serialize()
            .context("failed to serialize ibc sudo address")?;
        self.put_raw(IBC_SUDO_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_relayer_address<T: AddressBytes>(&mut self, address: &T) -> Result<()> {
        let bytes = StoredValue::Unit
            .serialize()
            .context("failed to serialize unit for ibc relayer address")?;
        self.put_raw(ibc_relayer_key(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_ibc_relayer_address<T: AddressBytes>(&mut self, address: &T) {
        self.delete(ibc_relayer_key(address));
    }

    #[instrument(skip_all)]
    fn put_ics20_withdrawal_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = StoredValue::Fee(fee.into())
            .serialize()
            .context("failed to serialize ics20 withdrawal base fee")?;
        self.put_raw(ICS20_WITHDRAWAL_BASE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        asset,
        Address,
    };
    use cnidarium::StateDelta;
    use ibc_types::core::channel::ChannelId;
    use insta::assert_snapshot;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };
    use crate::{
        address::StateWriteExt,
        ibc::state_ext::channel_balance_storage_key,
        test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
    };

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }
    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

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

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // can write new
        let mut address = [42u8; 20];
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
        address = [41u8; 20];
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
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // unset address returns false
        let address = astria_address(&[42u8; 20]);
        assert!(
            !state
                .is_ibc_relayer(address)
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

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // can write
        let address = astria_address(&[42u8; 20]);
        state.put_ibc_relayer_address(&address).unwrap();
        assert!(
            state
                .is_ibc_relayer(address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can delete
        state.delete_ibc_relayer_address(&address);
        assert!(
            !state
                .is_ibc_relayer(address)
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

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        // can write
        let address = astria_address(&[42u8; 20]);
        state.put_ibc_relayer_address(&address).unwrap();
        assert!(
            state
                .is_ibc_relayer(address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can write multiple
        let address_1 = astria_address(&[41u8; 20]);
        state.put_ibc_relayer_address(&address_1).unwrap();
        assert!(
            state
                .is_ibc_relayer(address_1)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "additional stored relayer address could not be verified"
        );
        assert!(
            state
                .is_ibc_relayer(address)
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
        let asset = asset_0();

        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, &asset)
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
        let asset = asset_0();
        let mut amount = 10u128;

        // write initial
        state
            .put_ibc_channel_balance(&channel, &asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );

        // can update
        amount = 20u128;
        state
            .put_ibc_channel_balance(&channel, &asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_multiple_assets() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let channel = ChannelId::new(0u64);
        let asset_0 = asset_0();
        let asset_1 = asset_1();
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state
            .put_ibc_channel_balance(&channel, &asset_0, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state
            .put_ibc_channel_balance(&channel, &asset_1, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, &asset_0)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel, &asset_1)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_multiple_channels() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let channel_0 = ChannelId::new(0u64);
        let channel_1 = ChannelId::new(1u64);
        let asset = asset_0();
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state
            .put_ibc_channel_balance(&channel_0, &asset, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state
            .put_ibc_channel_balance(&channel_1, &asset, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel_0, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state
                .get_ibc_channel_balance(&channel_1, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn ics20_withdrawal_base_fee_round_trip() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_ics20_withdrawal_base_fee(123).unwrap();
        let retrieved_fee = state.get_ics20_withdrawal_base_fee().await.unwrap();
        assert_eq!(retrieved_fee, 123);
    }

    #[test]
    fn storage_keys_have_not_changed() {
        let channel = ChannelId::new(5);
        let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap();

        assert_snapshot!(super::ibc_relayer_key(&address));

        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            channel_balance_storage_key(&channel, &asset),
            channel_balance_storage_key(&channel, &asset.to_ibc_prefixed()),
        );
        assert_snapshot!(channel_balance_storage_key(&channel, &asset));
    }
}
