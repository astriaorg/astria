use std::{
    borrow::Cow,
    fmt::Display,
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::primitive::v1::asset;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
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
use futures::Stream;
use pin_project_lite::pin_project;
use tracing::instrument;

use super::storage::{
    self,
    keys::{
        self,
        extract_asset_from_key,
    },
};
use crate::{
    accounts::AddressBytes,
    storage::StoredValue,
};

pin_project! {
    /// A stream of IBC prefixed assets for a given account.
    pub(crate) struct AccountAssetsStream<St> {
        #[pin]
        underlying: St,
    }
}

impl<St> Stream for AccountAssetsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<String>>,
{
    type Item = Result<asset::IbcPrefixed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let key = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(key)) => key,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(
                    anyhow_to_eyre(err).wrap_err("failed reading from state")
                )));
            }
            None => return Poll::Ready(None),
        };
        Poll::Ready(Some(extract_asset_from_key(&key).with_context(|| {
            format!("failed to extract IBC prefixed asset from key `{key}`")
        })))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssetBalance {
    pub(crate) asset: asset::IbcPrefixed,
    pub(crate) balance: u128,
}

pin_project! {
    /// A stream of IBC prefixed assets and their balances for a given account.
    pub(crate) struct AccountAssetBalancesStream<St> {
        #[pin]
        underlying: St,
    }
}

impl<St> Stream for AccountAssetBalancesStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<(String, Vec<u8>)>>,
{
    type Item = Result<AssetBalance>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let (key, bytes) = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(tup)) => tup,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(
                    anyhow_to_eyre(err).wrap_err("failed reading from state")
                )));
            }
            None => return Poll::Ready(None),
        };
        let asset = match extract_asset_from_key(&key)
            .with_context(|| format!("failed to extract IBC prefixed asset from key `{key}`"))
        {
            Err(e) => return Poll::Ready(Some(Err(e))),
            Ok(asset) => asset,
        };
        let balance = StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Balance::try_from(value).map(u128::from))
            .context("invalid balance bytes")?;
        Poll::Ready(Some(Ok(AssetBalance {
            asset,
            balance,
        })))
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead + crate::assets::StateReadExt {
    #[instrument(skip_all)]
    fn account_asset_keys<T: AddressBytes>(
        &self,
        address: &T,
    ) -> AccountAssetsStream<Self::PrefixKeysStream> {
        let prefix = keys::balance_prefix(address);
        AccountAssetsStream {
            underlying: self.prefix_keys(&prefix),
        }
    }

    #[instrument(skip_all)]
    fn account_asset_balances<T: AddressBytes>(
        &self,
        address: &T,
    ) -> AccountAssetBalancesStream<Self::PrefixRawStream> {
        let prefix = keys::balance_prefix(address);
        AccountAssetBalancesStream {
            underlying: self.prefix_raw(&prefix),
        }
    }

    #[instrument(skip_all, fields(address = %address.display_address(), %asset), err)]
    async fn get_account_balance<'a, TAddress, TAsset>(
        &self,
        address: &TAddress,
        asset: &'a TAsset,
    ) -> Result<u128>
    where
        TAddress: AddressBytes,
        TAsset: Sync + Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let Some(bytes) = self
            .get_raw(&keys::balance(address, asset))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw account balance from state")?
        else {
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Balance::try_from(value).map(u128::from))
            .wrap_err("invalid balance bytes")
    }

    #[instrument(skip_all)]
    async fn get_account_nonce<T: AddressBytes>(&self, address: &T) -> Result<u32> {
        let bytes = self
            .get_raw(&keys::nonce(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw account nonce from state")?;
        let Some(bytes) = bytes else {
            // the account has not yet been initialized; return 0
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Nonce::try_from(value).map(u32::from))
            .wrap_err("invalid nonce bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all, fields(address = %address.display_address(), %asset, balance), err)]
    fn put_account_balance<'a, TAddress, TAsset>(
        &mut self,
        address: &TAddress,
        asset: &'a TAsset,
        balance: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let bytes = StoredValue::from(storage::Balance::from(balance))
            .serialize()
            .wrap_err("failed to serialize balance")?;
        self.put_raw(keys::balance(address, asset), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_account_nonce<T: AddressBytes>(&mut self, address: &T, nonce: u32) -> Result<()> {
        let bytes = StoredValue::from(storage::Nonce::from(nonce))
            .serialize()
            .wrap_err("failed to serialize nonce")?;
        self.put_raw(keys::nonce(address), bytes);
        Ok(())
    }

    #[instrument(skip_all, fields(address = %address.display_address(), %asset, amount), err)]
    async fn increase_balance<'a, TAddress, TAsset>(
        &mut self,
        address: &TAddress,
        asset: &'a TAsset,
        amount: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Sync + Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let balance = self
            .get_account_balance(address, asset)
            .await
            .wrap_err("failed to get account balance")?;
        self.put_account_balance(
            address,
            asset,
            balance
                .checked_add(amount)
                .ok_or_eyre("failed to update account balance due to overflow")?,
        )
        .wrap_err("failed to store updated account balance in database")?;
        Ok(())
    }

    #[instrument(skip_all, fields(address = %address.display_address(), %asset, amount))]
    async fn decrease_balance<'a, TAddress, TAsset>(
        &mut self,
        address: &TAddress,
        asset: &'a TAsset,
        amount: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Sync + Display,
        &'a TAsset: Into<Cow<'a, asset::IbcPrefixed>>,
    {
        let balance = self
            .get_account_balance(address, asset)
            .await
            .wrap_err("failed to get account balance")?;
        self.put_account_balance(
            address,
            asset,
            balance
                .checked_sub(amount)
                .ok_or_eyre("subtracting from account balance failed due to insufficient funds")?,
        )
        .wrap_err("failed to store updated account balance in database")?;
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use futures::TryStreamExt as _;

    use super::*;
    use crate::{
        assets::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        benchmark_and_test_utils::{
            astria_address,
            nria,
        },
        storage::Storage,
    };

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    fn asset_2() -> asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[tokio::test]
    async fn get_account_nonce_uninitialized_returns_zero() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 0u32;

        // uninitialized accounts return zero
        assert_eq!(
            state_delta
                .get_account_nonce(&address)
                .await
                .expect("getting a non-initialized account's nonce should not fail"),
            nonce_expected,
            "returned nonce for non-initialized address was not zero"
        );
    }

    #[tokio::test]
    async fn get_account_nonce_get_nonce_simple() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 0u32;

        // can write new
        state_delta
            .put_account_nonce(&address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state_delta
                .get_account_nonce(&address)
                .await
                .expect("a nonce was written and must exist inside the database"),
            nonce_expected,
            "stored nonce was not what was expected"
        );

        // can rewrite with new value
        let nonce_expected = 1u32;
        state_delta
            .put_account_nonce(&address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state_delta
                .get_account_nonce(&address)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected,
            "updated nonce was not what was expected"
        );
    }

    #[tokio::test]
    async fn get_account_nonce_get_nonce_complex() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 2u32;

        // can write new
        state_delta
            .put_account_nonce(&address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state_delta
                .get_account_nonce(&address)
                .await
                .expect("a nonce was written and must exist inside the database"),
            nonce_expected,
            "stored nonce was not what was expected"
        );

        // writing additional account preserves first account's values
        let address_1 = astria_address(&[41u8; 20]);
        let nonce_expected_1 = 3u32;

        state_delta
            .put_account_nonce(&address_1, nonce_expected_1)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state_delta
                .get_account_nonce(&address_1)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected_1,
            "additional account's nonce was not what was expected"
        );
        assert_eq!(
            state_delta
                .get_account_nonce(&address)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected,
            "writing to a different account's nonce should not affect a different account's nonce"
        );
    }

    #[tokio::test]
    async fn get_account_balance_uninitialized_returns_zero() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_expected = 0u128;

        // non-initialized accounts return zero
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting a non-initialized asset balance should not fail"),
            amount_expected,
            "returned balance for non-initialized asset balance was not zero"
        );
    }

    #[tokio::test]
    async fn get_account_balance_simple() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let mut amount_expected = 1u128;

        state_delta
            .put_account_balance(&address, &asset, amount_expected)
            .expect("putting an account balance should not fail");

        // can initialize
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset balance did not match expected"
        );

        // can update balance
        amount_expected = 2u128;

        state_delta
            .put_account_balance(&address, &asset, amount_expected)
            .expect("putting an asset balance for an account should not fail");

        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected"
        );
    }

    #[tokio::test]
    async fn get_account_balance_multiple_accounts() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_expected = 1u128;

        state_delta
            .put_account_balance(&address, &asset, amount_expected)
            .expect("putting an account balance should not fail");

        // able to write to account's storage
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected"
        );

        // writing to other accounts does not affect original account
        // create needed variables
        let address_1 = astria_address(&[41u8; 20]);
        let amount_expected_1 = 2u128;

        state_delta
            .put_account_balance(&address_1, &asset, amount_expected_1)
            .expect("putting an account balance should not fail");
        assert_eq!(
            state_delta
                .get_account_balance(&address_1, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_1,
            "returned balance for an asset did not match expected, changed during different \
             account update"
        );
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected, changed during different \
             account update"
        );
    }

    #[tokio::test]
    async fn get_account_balance_multiple_assets() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset_0 = asset_0();
        let asset_1 = asset_1();
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;

        state_delta
            .put_account_balance(&address, &asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state_delta
            .put_account_balance(&address, &asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");

        // wrote correct balances
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset_0)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_0,
            "returned balance for an asset did not match expected"
        );
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset_1)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_1,
            "returned balance for an asset did not match expected"
        );
    }

    #[tokio::test]
    async fn account_asset_balances_uninitialized_ok() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);

        // see that call was ok
        let stream = state_delta.account_asset_balances(&address);

        // Collect the stream into a vector
        let balances: Vec<_> = stream
            .try_collect()
            .await
            .expect("Stream collection should not fail");

        // Assert that the vector is empty
        assert!(
            balances.is_empty(),
            "Expected no balances for uninitialized account"
        );
    }

    #[tokio::test]
    async fn account_asset_balances() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // native account should work with ibc too
        state_delta.put_native_asset(nria()).unwrap();

        let asset_0 = state_delta.get_native_asset().await.unwrap().unwrap();
        let asset_1 = asset_1();
        let asset_2 = asset_2();

        // also need to add assets to the ibc state
        state_delta
            .put_ibc_asset(asset_0.clone())
            .expect("should be able to call other trait method on state object");
        state_delta
            .put_ibc_asset(asset_1.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");
        state_delta
            .put_ibc_asset(asset_2.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;
        let amount_expected_2 = 3u128;

        // add balances to the account
        state_delta
            .put_account_balance(&address, &asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state_delta
            .put_account_balance(&address, &asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");
        state_delta
            .put_account_balance(&address, &asset_2, amount_expected_2)
            .expect("putting an account balance should not fail");

        let mut balances = state_delta
            .account_asset_balances(&address)
            .try_collect::<Vec<_>>()
            .await
            .expect("should not fail");
        balances.sort_by_key(|k| k.asset.to_string());

        assert_eq!(
            balances.first().unwrap().balance,
            amount_expected_1,
            "returned value for ibc asset_1 does not match"
        );
        assert_eq!(
            balances.get(1).unwrap().balance,
            amount_expected_0,
            "returned value for ibc asset_0 does not match"
        );
        assert_eq!(
            balances.get(2).unwrap().balance,
            amount_expected_2,
            "returned value for ibc asset_2 does not match"
        );
        assert_eq!(balances.len(), 3, "should only return existing values");
    }

    #[tokio::test]
    async fn increase_balance_from_uninitialized() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        state_delta
            .increase_balance(&address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        state_delta
            .increase_balance(&address, &asset, amount_increase)
            .await
            .expect("increasing account balance for initialized account should be ok");

        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase * 2,
            "returned balance for an asset balance did not match expected"
        );
    }

    #[tokio::test]
    async fn decrease_balance_enough_funds() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        state_delta
            .increase_balance(&address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        // decrease balance
        state_delta
            .decrease_balance(&address, &asset, amount_increase)
            .await
            .expect("decreasing account balance for initialized account should be ok");

        assert_eq!(
            state_delta
                .get_account_balance(&address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            0,
            "returned balance for an asset balance did not match expected"
        );
    }

    #[tokio::test]
    async fn decrease_balance_not_enough_funds() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        // give initial balance
        state_delta
            .increase_balance(&address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // decrease balance
        let _ = state_delta
            .decrease_balance(&address, &asset, amount_increase + 1)
            .await
            .expect_err("should not be able to subtract larger balance than what existed");
    }
}
