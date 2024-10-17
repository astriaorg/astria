use std::{
    borrow::Cow,
    fmt::Display,
};

use astria_core::primitive::v1::{
    asset,
    ADDRESS_LEN,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
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

use super::storage::{
    self,
    keys,
};
use crate::{
    accounts::AddressBytes,
    storage::StoredValue,
};

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all, fields(%channel, %asset), err)]
    async fn get_ibc_channel_balance<'a, TAsset>(
        &self,
        channel: &ChannelId,
        asset: &'a TAsset,
    ) -> Result<u128>
    where
        TAsset: Sync + Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let Some(bytes) = self
            .get_raw(&keys::channel_balance(channel, asset))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading ibc channel balance from state")?
        else {
            debug!("ibc channel balance not found, returning 0");
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Balance::try_from(value).map(u128::from))
            .wrap_err("invalid ibc channel balance bytes")
    }

    #[instrument(skip_all)]
    async fn get_ibc_sudo_address(&self) -> Result<[u8; ADDRESS_LEN]> {
        let Some(bytes) = self
            .get_raw(keys::IBC_SUDO)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw ibc sudo address from state")?
        else {
            // ibc sudo key must be set
            bail!("ibc sudo address not found");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::AddressBytes::try_from(value).map(<[u8; ADDRESS_LEN]>::from))
            .wrap_err("invalid ibc sudo address bytes")
    }

    #[instrument(skip_all)]
    async fn is_ibc_relayer<T: AddressBytes>(&self, address: T) -> Result<bool> {
        Ok(self
            .get_raw(&keys::ibc_relayer(&address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read ibc relayer key from state")?
            .is_some())
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all, fields(%channel, %asset, balance), err)]
    fn put_ibc_channel_balance<'a, TAsset>(
        &mut self,
        channel: &ChannelId,
        asset: &'a TAsset,
        balance: u128,
    ) -> Result<()>
    where
        TAsset: Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let bytes = StoredValue::from(storage::Balance::from(balance))
            .serialize()
            .wrap_err("failed to serialize ibc channel balance")?;
        self.put_raw(keys::channel_balance(channel, asset), bytes);
        Ok(())
    }

    #[instrument(skip_all, fields(%channel, %asset, amount), err)]
    async fn decrease_ibc_channel_balance<'a, TAsset>(
        &mut self,
        channel: &ChannelId,
        asset: &'a TAsset,
        amount: u128,
    ) -> Result<()>
    where
        TAsset: Sync + Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let old_balance = self
            .get_ibc_channel_balance(channel, asset)
            .await
            .wrap_err("failed to get ibc channel balance")?;

        let new_balance = old_balance
            .checked_sub(amount)
            .ok_or_eyre("insufficient funds on ibc channel")?;

        self.put_ibc_channel_balance(channel, asset, new_balance)
            .wrap_err("failed to write new balance to ibc channel")
    }

    #[instrument(skip_all)]
    fn put_ibc_sudo_address<T: AddressBytes>(&mut self, address: T) -> Result<()> {
        let bytes = StoredValue::from(storage::AddressBytes::from(&address))
            .serialize()
            .wrap_err("failed to serialize ibc sudo address")?;
        self.put_raw(keys::IBC_SUDO.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_ibc_relayer_address<T: AddressBytes>(&mut self, address: &T) -> Result<()> {
        let bytes = StoredValue::Unit
            .serialize()
            .wrap_err("failed to serialize unit for ibc relayer address")?;
        self.put_raw(keys::ibc_relayer(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_ibc_relayer_address<T: AddressBytes>(&mut self, address: &T) {
        self.delete(keys::ibc_relayer(address));
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        storage::Storage,
    };

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn get_ibc_sudo_address_fails_if_not_set() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        // should fail if not set
        let _ = state_delta
            .get_ibc_sudo_address()
            .await
            .expect_err("sudo address should be set");
    }

    #[tokio::test]
    async fn put_ibc_sudo_address() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // can write new
        let mut address = [42u8; 20];
        state_delta
            .put_ibc_sudo_address(address)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state_delta
                .get_ibc_sudo_address()
                .await
                .expect("a sudo address was written and must exist inside the database"),
            address,
            "stored sudo address was not what was expected"
        );

        // can rewrite with new value
        address = [41u8; 20];
        state_delta
            .put_ibc_sudo_address(address)
            .expect("writing sudo address should not fail");
        assert_eq!(
            state_delta
                .get_ibc_sudo_address()
                .await
                .expect("sudo address was written and must exist inside the database"),
            address,
            "updated sudo address was not what was expected"
        );
    }

    #[tokio::test]
    async fn is_ibc_relayer_ok_if_not_set() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // unset address returns false
        let address = astria_address(&[42u8; 20]);
        assert!(
            !state_delta
                .is_ibc_relayer(address)
                .await
                .expect("calls to properly formatted addresses should not fail"),
            "inputted address should've returned false"
        );
    }

    #[tokio::test]
    async fn delete_ibc_relayer_address() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // can write
        let address = astria_address(&[42u8; 20]);
        state_delta.put_ibc_relayer_address(&address).unwrap();
        assert!(
            state_delta
                .is_ibc_relayer(address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can delete
        state_delta.delete_ibc_relayer_address(&address);
        assert!(
            !state_delta
                .is_ibc_relayer(address)
                .await
                .expect("calls on unset addresses should not fail"),
            "relayer address was not deleted as was intended"
        );
    }

    #[tokio::test]
    async fn put_ibc_relayer_address() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // can write
        let address = astria_address(&[42u8; 20]);
        state_delta.put_ibc_relayer_address(&address).unwrap();
        assert!(
            state_delta
                .is_ibc_relayer(address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "stored relayer address could not be verified"
        );

        // can write multiple
        let address_1 = astria_address(&[41u8; 20]);
        state_delta.put_ibc_relayer_address(&address_1).unwrap();
        assert!(
            state_delta
                .is_ibc_relayer(address_1)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "additional stored relayer address could not be verified"
        );
        assert!(
            state_delta
                .is_ibc_relayer(address)
                .await
                .expect("a relayer address was written and must exist inside the database"),
            "original stored relayer address could not be verified"
        );
    }

    #[tokio::test]
    async fn get_ibc_channel_balance_unset_ok() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        let channel = ChannelId::new(0u64);
        let asset = asset_0();

        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            0u128,
            "unset asset and channel should return zero"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_simple() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let channel = ChannelId::new(0u64);
        let asset = asset_0();
        let mut amount = 10u128;

        // write initial
        state_delta
            .put_ibc_channel_balance(&channel, &asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );

        // can update
        amount = 20u128;
        state_delta
            .put_ibc_channel_balance(&channel, &asset, amount)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_multiple_assets() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let channel = ChannelId::new(0u64);
        let asset_0 = asset_0();
        let asset_1 = asset_1();
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state_delta
            .put_ibc_channel_balance(&channel, &asset_0, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state_delta
            .put_ibc_channel_balance(&channel, &asset_1, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel, &asset_0)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel, &asset_1)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }

    #[tokio::test]
    async fn put_ibc_channel_balance_multiple_channels() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let channel_0 = ChannelId::new(0u64);
        let channel_1 = ChannelId::new(1u64);
        let asset = asset_0();
        let amount_0 = 10u128;
        let amount_1 = 20u128;

        // write both
        state_delta
            .put_ibc_channel_balance(&channel_0, &asset, amount_0)
            .expect("should be able to set balance for channel and asset pair");
        state_delta
            .put_ibc_channel_balance(&channel_1, &asset, amount_1)
            .expect("should be able to set balance for channel and asset pair");
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel_0, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_0,
            "set balance for channel/asset pair not what was expected"
        );
        assert_eq!(
            state_delta
                .get_ibc_channel_balance(&channel_1, &asset)
                .await
                .expect("retrieving asset balance for channel should not fail"),
            amount_1,
            "set balance for channel/asset pair not what was expected"
        );
    }
}
