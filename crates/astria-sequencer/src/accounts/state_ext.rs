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
use hex::ToHex as _;
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

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all, fields(address=%address))]
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
                    denom: astria_core::primitive::v1::asset::Denom::from(
                        native_asset.base_denom().to_owned(),
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

    #[instrument(skip_all, fields(address=%address, asset_id=%asset))]
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

    #[instrument(skip_all, fields(address=%address))]
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
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_account_balance(
        &mut self,
        address: Address,
        asset: asset::Id,
        balance: u128,
    ) -> Result<()> {
        let bytes = borsh::to_vec(&Balance(balance)).context("failed to serialize balance")?;
        self.put_raw(balance_storage_key(address, asset), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_account_nonce(&mut self, address: Address, nonce: u32) -> Result<()> {
        let bytes = borsh::to_vec(&Nonce(nonce)).context("failed to serialize nonce")?;
        self.put_raw(nonce_storage_key(address), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn increase_balance(
        &mut self,
        address: Address,
        asset: asset::Id,
        amount: u128,
    ) -> Result<()> {
        let balance = self
            .get_account_balance(address, asset)
            .await
            .context("failed to get account balance")?;
        self.put_account_balance(
            address,
            asset,
            balance
                .checked_add(amount)
                .context("failed to update account balance due to overflow")?,
        )
        .context("failed to store updated account balance in database")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn decrease_balance(
        &mut self,
        address: Address,
        asset: asset::Id,
        amount: u128,
    ) -> Result<()> {
        let balance = self
            .get_account_balance(address, asset)
            .await
            .context("failed to get account balance")?;
        self.put_account_balance(
            address,
            asset,
            balance
                .checked_sub(amount)
                .context("subtracting from account balance failed due to insufficient funds")?,
        )
        .context("failed to store updated account balance in database")?;
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset::{
                Denom,
                Id,
                DEFAULT_NATIVE_ASSET_DENOM,
            },
            Address,
        },
        protocol::account::v1alpha1::AssetBalance,
    };
    use cnidarium::StateDelta;

    use super::{
        StateReadExt as _,
        StateWriteExt as _,
    };
    use crate::asset;

    #[tokio::test]
    async fn get_account_nonce_uninitialized_returns_zero() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // create needed variables
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
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
        let address_1 = Address::try_from_slice(&[41u8; 20]).unwrap();
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
        let mut amount_expected = 1u128;

        state
            .put_account_balance(address, asset, amount_expected)
            .expect("putting an account balance should not fail");

        // can initialize
        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset balance did not match expected"
        );

        // can update balance
        amount_expected = 2u128;

        state
            .put_account_balance(address, asset, amount_expected)
            .expect("putting an asset balance for an account should not fail");

        assert_eq!(
            state
                .get_account_balance(address, asset)
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
        let amount_expected = 1u128;

        state
            .put_account_balance(address, asset, amount_expected)
            .expect("putting an account balance should not fail");

        // able to write to account's storage
        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected,
            "returned balance for an asset did not match expected"
        );

        // writing to other accounts does not affect original account
        // create needed variables
        let address_1 = Address::try_from_slice(&[41u8; 20]).unwrap();
        let amount_expected_1 = 2u128;

        state
            .put_account_balance(address_1, asset, amount_expected_1)
            .expect("putting an account balance should not fail");
        assert_eq!(
            state
                .get_account_balance(address_1, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_1,
            "returned balance for an asset did not match expected, changed during different \
             account update"
        );
        assert_eq!(
            state
                .get_account_balance(address, asset)
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset_0 = Id::from_denom("asset_0");
        let asset_1 = Id::from_denom("asset_1");
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;

        state
            .put_account_balance(address, asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");

        // wrote correct balances
        assert_eq!(
            state
                .get_account_balance(address, asset_0)
                .await
                .expect("getting an asset balance should not fail"),
            amount_expected_0,
            "returned balance for an asset did not match expected"
        );
        assert_eq!(
            state
                .get_account_balance(address, asset_1)
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();

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
        crate::asset::initialize_native_asset(DEFAULT_NATIVE_ASSET_DENOM);

        let asset_0 = Id::from_denom(DEFAULT_NATIVE_ASSET_DENOM);
        let asset_1 = Id::from_denom("asset_1");
        let asset_2 = Id::from_denom("asset_2");

        // also need to add assets to the ibc state
        asset::state_ext::StateWriteExt::put_ibc_asset(
            &mut state,
            asset_0,
            &Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM),
        )
        .expect("should be able to call other trait method on state object");
        asset::state_ext::StateWriteExt::put_ibc_asset(
            &mut state,
            asset_1,
            &Denom::from_base_denom("asset_1"),
        )
        .expect("should be able to call other trait method on state object");
        asset::state_ext::StateWriteExt::put_ibc_asset(
            &mut state,
            asset_2,
            &Denom::from_base_denom("asset_2"),
        )
        .expect("should be able to call other trait method on state object");

        // create needed variables
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;
        let amount_expected_2 = 3u128;

        // add balances to the account
        state
            .put_account_balance(address, asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(address, asset_2, amount_expected_2)
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
                    denom: Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM),
                    balance: amount_expected_0,
                },
                AssetBalance {
                    denom: Denom::from_base_denom("asset_1"),
                    balance: amount_expected_1,
                },
                AssetBalance {
                    denom: Denom::from_base_denom("asset_2"),
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
        let amount_increase = 2u128;

        state
            .increase_balance(address, asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        state
            .increase_balance(address, asset, amount_increase)
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
        let amount_increase = 2u128;

        state
            .increase_balance(address, asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // correct balance was set
        assert_eq!(
            state
                .get_account_balance(address, asset)
                .await
                .expect("getting an asset balance should not fail"),
            amount_increase,
            "returned balance for an asset balance did not match expected"
        );

        // decrease balance
        state
            .decrease_balance(address, asset, amount_increase)
            .await
            .expect("decreasing account balance for initialized account should be ok");

        assert_eq!(
            state
                .get_account_balance(address, asset)
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
        let address = Address::try_from_slice(&[42u8; 20]).unwrap();
        let asset = Id::from_denom("asset_0");
        let amount_increase = 2u128;

        // give initial balance
        state
            .increase_balance(address, asset, amount_increase)
            .await
            .expect("increasing account balance for uninitialized account should be ok");

        // decrease balance
        state
            .decrease_balance(address, asset, amount_increase + 1)
            .await
            .expect_err("should not be able to subtract larger balance than what existed");
    }
}
