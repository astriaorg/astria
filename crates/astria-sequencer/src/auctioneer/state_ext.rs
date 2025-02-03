use astria_core::protocol::auctioneer::v1::EnshrinedAuctioneerEntry;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    debug,
    instrument,
    Level,
};

use super::storage::{
    self,
    keys,
};
use crate::{
    accounts::AddressBytes,
    address,
    storage::StoredValue,
};

#[async_trait]
pub(crate) trait StateReadExt: StateRead + address::StateReadExt {
    #[instrument(skip_all, fields(address = %address.display_address()))]
    async fn is_an_enshrined_auctioneer<T: AddressBytes>(&self, address: &T) -> Result<bool> {
        let maybe_id = self.get_enshrined_auctioneer_entry(address).await?;
        Ok(maybe_id.is_some())
    }

    #[instrument(skip_all, fields(address = %address.display_address()), err(level = Level::WARN))]
    async fn get_enshrined_auctioneer_entry<T: AddressBytes>(
        &self,
        address: &T,
    ) -> Result<Option<EnshrinedAuctioneerEntry>> {
        let Some(bytes) = self
            .get_raw(&keys::enshrined_auctioneer_key(address))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading raw enshrined auctioneer entry from state")?
        else {
            debug!("account enshrined auctioneer entry not found, returning None");
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::EnshrinedAuctioneerEntry::try_from(value).map(
                    |enshrined_auctioneer_entry| {
                        Some(EnshrinedAuctioneerEntry::from(enshrined_auctioneer_entry))
                    },
                )
            })
            .wrap_err("invalid enshrined auctioneer entry bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_enshrined_auctioneer_entry<T: AddressBytes>(
        &mut self,
        address: &T,
        enshrined_auctioneer_entry: EnshrinedAuctioneerEntry,
    ) -> Result<()> {
        let bytes = StoredValue::from(storage::EnshrinedAuctioneerEntry::from(
            &enshrined_auctioneer_entry,
        ))
        .serialize()
        .context("failed to serialize bridge account rollup id")?;
        self.put_raw(keys::enshrined_auctioneer_key(address), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn delete_enshrined_auctioneer_entry<T: AddressBytes>(&mut self, address: &T) -> Result<()> {
        self.delete(keys::enshrined_auctioneer_key(address));
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::asset;
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        auctioneer::state_ext::{
            StateReadExt,
            StateWriteExt,
        },
        benchmark_and_test_utils::astria_address,
    };

    fn asset_0() -> asset::Denom {
        "asset_0".parse().unwrap()
    }

    fn asset_1() -> asset::Denom {
        "asset_1".parse().unwrap()
    }

    #[tokio::test]
    async fn get_enshrined_auctioneer_entry_uninitialized_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let address = astria_address(&[42u8; 20]);

        // uninitialized ok
        assert_eq!(
            state.get_enshrined_auctioneer_entry(&address).await.expect(
                "call to get enshrined auctioneer entry should not fail for uninitialized \
                 addresses"
            ),
            Option::None,
            "stored enshrined auctioneer entry for bridge not what was expected"
        );
    }

    #[tokio::test]
    async fn put_enshrined_auctioneer_entry() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let auctioneer_address = astria_address(&[42u8; 20]);
        let staker_address = astria_address(&[43u8; 20]);

        let enshrined_auctioneer_entry = EnshrinedAuctioneerEntry {
            auctioneer_address,
            staker_address,
            staked_amount: 100,
            asset: asset_0(),
            fee_asset: asset_1(),
        };

        // can write new
        state
            .put_enshrined_auctioneer_entry(&auctioneer_address, enshrined_auctioneer_entry.clone())
            .unwrap();
        assert_eq!(
            state
                .get_enshrined_auctioneer_entry(&auctioneer_address)
                .await
                .expect(
                    "an enshrined auctioneer entry was written and must exist inside the database"
                )
                .expect("expecting return value"),
            enshrined_auctioneer_entry,
            "stored enshrined auctioneer entry not what was expected"
        );

        let is_an_enshrined_auctioneer_entry = state
            .is_an_enshrined_auctioneer(&auctioneer_address)
            .await
            .expect("call to is an enshrined auctioneer entry should not fail");
        assert!(is_an_enshrined_auctioneer_entry, "expecting return value");
    }

    #[tokio::test]
    async fn delete_enshrined_auctioneer_entry() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let auctioneer_address = astria_address(&[42u8; 20]);
        let staker_address = astria_address(&[43u8; 20]);

        let enshrined_auctioneer_entry = EnshrinedAuctioneerEntry {
            auctioneer_address,
            staker_address,
            staked_amount: 100,
            asset: asset_0(),
            fee_asset: asset_1(),
        };

        // can write new
        state
            .put_enshrined_auctioneer_entry(&auctioneer_address, enshrined_auctioneer_entry.clone())
            .unwrap();
        assert_eq!(
            state
                .get_enshrined_auctioneer_entry(&auctioneer_address)
                .await
                .expect(
                    "an enshrined auctioneer entry was written and must exist inside the database"
                )
                .expect("expecting return value"),
            enshrined_auctioneer_entry,
            "stored enshrined auctioneer entry not what was expected"
        );

        let is_an_enshrined_auctioneer_entry = state
            .is_an_enshrined_auctioneer(&auctioneer_address)
            .await
            .expect("call to is an enshrined auctioneer entry should not fail");
        assert!(is_an_enshrined_auctioneer_entry, "expecting return value");

        state
            .delete_enshrined_auctioneer_entry(&auctioneer_address)
            .expect("call to delete enshrined auctioneer entry should not fail");

        assert_eq!(
            state
                .get_enshrined_auctioneer_entry(&auctioneer_address)
                .await
                .expect(
                    "call to get enshrined auctioneer entry should not fail for uninitialized \
                     addresses"
                ),
            Option::None,
            "stored enshrined auctioneer entry for bridge not what was expected"
        );
    }
}
