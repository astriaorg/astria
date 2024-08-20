use anyhow::{
    Context,
    Result,
};
use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::account::v1alpha1::AssetBalance,
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
use tracing::instrument;

use super::AddressBytes;

/// Newtype wrapper to read and write a u32 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Nonce(u32);

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Balance(u128);

/// Newtype wrapper to read and write a u128 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Fee(u128);

const ACCOUNTS_PREFIX: &str = "accounts";
const TRANSFER_BASE_FEE_STORAGE_KEY: &str = "transferfee";

struct StorageKey<'a, T>(&'a T);
impl<'a, T: AddressBytes> std::fmt::Display for StorageKey<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(ACCOUNTS_PREFIX)?;
        f.write_str("/")?;
        for byte in self.0.address_bytes() {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

fn balance_storage_key<TAddress: AddressBytes, TAsset: Into<asset::IbcPrefixed>>(
    address: TAddress,
    asset: TAsset,
) -> String {
    format!(
        "{}/balance/{}",
        StorageKey(&address),
        crate::storage_keys::hunks::Asset::from(asset)
    )
}

fn nonce_storage_key<T: AddressBytes>(address: T) -> String {
    format!("{}/nonce", StorageKey(&address))
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead + crate::assets::StateReadExt {
    #[instrument(skip_all)]
    async fn get_account_balances(&self, address: Address) -> Result<Vec<AssetBalance>> {
        let prefix = format!("{}/balance/", StorageKey(&address));
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

            let asset = key
                .strip_prefix(&prefix)
                .context("failed to strip prefix from account balance key")?
                .parse::<crate::storage_keys::hunks::Asset>()
                .context("failed to parse storage key suffix as address hunk")?
                .get();

            let Balance(balance) =
                Balance::try_from_slice(&value).context("invalid balance bytes")?;

            let native_asset = self
                .get_native_asset()
                .await
                .context("failed to read native asset from state")?;
            if asset == native_asset.to_ibc_prefixed() {
                balances.push(AssetBalance {
                    denom: native_asset.into(),
                    balance,
                });
                continue;
            }

            let denom = self
                .map_ibc_to_trace_prefixed_asset(asset)
                .await
                .context("failed to get ibc asset denom")?
                .context("asset denom not found when user has balance of it; this is a bug")?
                .into();
            balances.push(AssetBalance {
                denom,
                balance,
            });
        }
        Ok(balances)
    }

    #[instrument(skip_all)]
    async fn get_account_balance<'a, TAddress, TAsset>(
        &self,
        address: TAddress,
        asset: TAsset,
    ) -> Result<u128>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        let Some(bytes) = self
            .get_raw(&balance_storage_key(address, asset))
            .await
            .context("failed reading raw account balance from state")?
        else {
            return Ok(0);
        };
        let Balance(balance) = Balance::try_from_slice(&bytes).context("invalid balance bytes")?;
        Ok(balance)
    }

    #[instrument(skip_all)]
    async fn get_account_nonce<T: AddressBytes>(&self, address: T) -> Result<u32> {
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

    #[instrument(skip_all)]
    async fn get_transfer_base_fee(&self) -> Result<u128> {
        let bytes = self
            .get_raw(TRANSFER_BASE_FEE_STORAGE_KEY)
            .await
            .context("failed reading raw transfer base fee from state")?;
        let Some(bytes) = bytes else {
            return Err(anyhow::anyhow!("transfer base fee not set"));
        };

        let Fee(fee) = Fee::try_from_slice(&bytes).context("invalid fee bytes")?;
        Ok(fee)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_account_balance<TAddress, TAsset>(
        &mut self,
        address: TAddress,
        asset: TAsset,
        balance: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        let bytes = borsh::to_vec(&Balance(balance)).context("failed to serialize balance")?;
        self.put_raw(balance_storage_key(address, asset), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_account_nonce<T: AddressBytes>(&mut self, address: T, nonce: u32) -> Result<()> {
        let bytes = borsh::to_vec(&Nonce(nonce)).context("failed to serialize nonce")?;
        self.put_raw(nonce_storage_key(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    async fn increase_balance<TAddress, TAsset>(
        &mut self,
        address: TAddress,
        asset: TAsset,
        amount: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        let asset = asset.into();
        let balance = self
            .get_account_balance(&address, asset)
            .await
            .context("failed to get account balance")?;
        self.put_account_balance(
            &address,
            asset,
            balance
                .checked_add(amount)
                .context("failed to update account balance due to overflow")?,
        )
        .context("failed to store updated account balance in database")?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn decrease_balance<TAddress, TAsset>(
        &mut self,
        address: TAddress,
        asset: TAsset,
        amount: u128,
    ) -> Result<()>
    where
        TAddress: AddressBytes,
        TAsset: Into<asset::IbcPrefixed> + std::fmt::Display + Send,
    {
        let asset = asset.into();
        let balance = self
            .get_account_balance(&address, asset)
            .await
            .context("failed to get account balance")?;
        self.put_account_balance(
            &address,
            asset,
            balance
                .checked_sub(amount)
                .context("subtracting from account balance failed due to insufficient funds")?,
        )
        .context("failed to store updated account balance in database")?;
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_transfer_base_fee(&mut self, fee: u128) -> Result<()> {
        let bytes = borsh::to_vec(&Fee(fee)).context("failed to serialize fee")?;
        self.put_raw(TRANSFER_BASE_FEE_STORAGE_KEY.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::Address,
        protocol::account::v1alpha1::AssetBalance,
    };
    use cnidarium::StateDelta;
    use insta::assert_snapshot;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };
    use crate::{
        accounts::state_ext::{
            balance_storage_key,
            nonce_storage_key,
        },
        assets::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        test_utils::{
            astria_address,
            nria,
        },
    };

    fn asset_0() -> astria_core::primitive::v1::asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> astria_core::primitive::v1::asset::Denom {
        "asset_1".parse().unwrap()
    }
    fn asset_2() -> astria_core::primitive::v1::asset::Denom {
        "asset_2".parse().unwrap()
    }

    #[tokio::test]
    async fn get_account_nonce_uninitialized_returns_zero() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 0u32;

        // uninitialized accounts return zero
        assert_eq!(
            state
                .get_account_nonce(address)
                .await
                .expect("getting a non-initialized account's nonce should not fail"),
            nonce_expected,
            "returned nonce for non-initialized address was not zero"
        );
    }

    #[tokio::test]
    async fn get_account_nonce_get_nonce_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 0u32;

        // can write new
        state
            .put_account_nonce(address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state
                .get_account_nonce(address)
                .await
                .expect("a nonce was written and must exist inside the database"),
            nonce_expected,
            "stored nonce was not what was expected"
        );

        // can rewrite with new value
        let nonce_expected = 1u32;
        state
            .put_account_nonce(address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state
                .get_account_nonce(address)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected,
            "updated nonce was not what was expected"
        );
    }

    #[tokio::test]
    async fn get_account_nonce_get_nonce_complex() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let nonce_expected = 2u32;

        // can write new
        state
            .put_account_nonce(address, nonce_expected)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state
                .get_account_nonce(address)
                .await
                .expect("a nonce was written and must exist inside the database"),
            nonce_expected,
            "stored nonce was not what was expected"
        );

        // writing additional account preserves first account's values
        let address_1 = astria_address(&[41u8; 20]);
        let nonce_expected_1 = 3u32;

        state
            .put_account_nonce(address_1, nonce_expected_1)
            .expect("putting an account nonce should not fail");
        assert_eq!(
            state
                .get_account_nonce(address_1)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected_1,
            "additional account's nonce was not what was expected"
        );
        assert_eq!(
            state
                .get_account_nonce(address)
                .await
                .expect("a new nonce was written and must exist inside the database"),
            nonce_expected,
            "writing to a different account's nonce should not affect a different account's nonce"
        );
    }

    #[tokio::test]
    async fn get_account_balance_uninitialized_returns_zero() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_expected = 0u128;

        // non-initialized accounts return zero
        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting a non-initialized asset balance should not fail"),
            amount_expected,
            "returned balance for non-initialized asset balance was not zero"
        );
    }

    #[tokio::test]
    async fn get_account_balance_simple() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let mut amount_expected = 1u128;

        state
            .put_account_balance(address, &asset, amount_expected)
            .expect("putting an account balance should not fail");

        // can initialize
        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset balance did not match expected"
        );

        // can update balance
        amount_expected = 2u128;

        state
            .put_account_balance(address, &asset, amount_expected)
            .expect("putting an asset balance for an account should not fail");

        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected"
        );
    }

    #[tokio::test]
    async fn get_account_balance_multiple_accounts() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_expected = 1u128;

        state
            .put_account_balance(address, &asset, amount_expected)
            .expect("putting an account balance should not fail");

        // able to write to account's storage
        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected"
        );

        // writing to other accounts does not affect original account
        // create needed variables
        let address_1 = astria_address(&[41u8; 20]);
        let amount_expected_1 = 2u128;

        state
            .put_account_balance(address_1, &asset, amount_expected_1)
            .expect("putting an account balance should not fail");
        assert_eq!(
            state
                .get_account_balance(address_1, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_1,
            "returned balance for an asset did not match expected, changed during different \
             account update"
        );
        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected, changed during different \
             account update"
        );
    }

    #[tokio::test]
    async fn get_account_balance_multiple_assets() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset_0 = asset_0();
        let asset_1 = asset_1();
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;

        state
            .put_account_balance(address, &asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, &asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");

        // wrote correct balances
        assert_eq!(
            state
                .get_account_balance(address, &asset_0)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_0,
            "returned balance for an asset did not match expected"
        );
        assert_eq!(
            state
                .get_account_balance(address, &asset_1)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_1,
            "returned balance for an asset did not match expected"
        );
    }

    #[tokio::test]
    async fn get_account_balances_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);

        // see that call was ok
        let balances = state
            .get_account_balances(address)
            .await
            .expect("retrieving account balances should not fail");
        assert_eq!(balances, vec![]);
    }

    #[tokio::test]
    async fn get_account_balances() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // need to set native asset in order to use `get_account_balances()`
        state.put_native_asset(&nria());

        let asset_0 = state.get_native_asset().await.unwrap();
        let asset_1 = asset_1();
        let asset_2 = asset_2();

        // also need to add assets to the ibc state
        state
            .put_ibc_asset(&asset_0.clone())
            .expect("should be able to call other trait method on state object");
        state
            .put_ibc_asset(&asset_1.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");
        state
            .put_ibc_asset(&asset_2.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;
        let amount_expected_2 = 3u128;

        // add balances to the account
        state
            .put_account_balance(address, asset_0.clone(), amount_expected_0)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, &asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, &asset_2, amount_expected_2)
            .expect("putting an account balance should not fail");

        let mut balances = state
            .get_account_balances(address)
            .await
            .expect("retrieving account balances should not fail");
        balances.sort_by(|a, b| a.balance.cmp(&b.balance));
        assert_eq!(
            balances,
            vec![
                AssetBalance {
                    denom: asset_0.into(),
                    balance: amount_expected_0,
                },
                AssetBalance {
                    denom: asset_1.clone(),
                    balance: amount_expected_1,
                },
                AssetBalance {
                    denom: asset_2.clone(),
                    balance: amount_expected_2,
                },
            ]
        );
    }

    #[tokio::test]
    async fn increase_balance_from_uninitialized() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        state
            .increase_balance(address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        state
            .increase_balance(address, &asset, amount_increase)
            .await
            .expect("increasing account balance for initialized account should be ok");

        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase * 2,
            "returned balance for an asset balance did not match expected"
        );
    }

    #[tokio::test]
    async fn decrease_balance_enough_funds() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        state
            .increase_balance(address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        // decrease balance
        state
            .decrease_balance(address, &asset, amount_increase)
            .await
            .expect("decreasing account balance for initialized account should be ok");

        assert_eq!(
            state
                .get_account_balance(address, &asset)
                .await
                .expect("getting an asset balance should not fail"),
            0,
            "returned balance for an asset balance did not match expected"
        );
    }

    #[tokio::test]
    async fn decrease_balance_not_enough_funds() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let asset = asset_0();
        let amount_increase = 2u128;

        // give initial balance
        state
            .increase_balance(address, &asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // decrease balance
        state
            .decrease_balance(address, &asset, amount_increase + 1)
            .await
            .expect_err("should not be able to subtract larger balance than what existed");
    }

    #[test]
    fn storage_keys_have_not_changed() {
        let address: Address = "astria1rsxyjrcm255ds9euthjx6yc3vrjt9sxrm9cfgm"
            .parse()
            .unwrap();
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        assert_eq!(
            balance_storage_key(address, &asset),
            balance_storage_key(address, asset.to_ibc_prefixed())
        );
        assert_snapshot!(balance_storage_key(address, asset));
        assert_snapshot!(nonce_storage_key(address));
    }
}
