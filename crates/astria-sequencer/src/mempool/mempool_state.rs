use std::collections::HashMap;

use astria_core::primitive::v1::asset;
use astria_eyre::eyre::Result;
use cnidarium::StateRead;
use tracing::instrument;

use crate::accounts::{
    AddressBytes,
    StateReadExt as _,
};

#[instrument(skip_all)]
pub(crate) async fn get_account_balances<S: StateRead, T: AddressBytes>(
    state: S,
    address: &T,
) -> Result<HashMap<asset::IbcPrefixed, u128>> {
    use futures::TryStreamExt as _;
    state
        .account_asset_balances(address)
        .map_ok(
            |crate::accounts::AssetBalance {
                 asset,
                 balance,
             }| (asset, balance),
        )
        // note: this relies on the IBC prefixed assets coming out of the stream to be unique
        .try_collect::<std::collections::HashMap<_, _>>()
        .await
}

#[cfg(test)]
mod tests {
    use asset::Denom;
    use cnidarium::StateDelta;

    use super::*;
    use crate::{
        accounts::StateWriteExt as _,
        assets::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        test_utils::{
            astria_address,
            nria,
        },
    };

    #[tokio::test]
    async fn test_get_account_balances() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // native account should work with ibc too
        state.put_native_asset(nria()).unwrap();

        let asset_0 = state.get_native_asset().await.unwrap();
        let asset_1: Denom = "asset_0".parse().unwrap();
        let asset_2: Denom = "asset_1".parse().unwrap();

        // also need to add assets to the ibc state
        state
            .put_ibc_asset(asset_0.clone())
            .expect("should be able to call other trait method on state object");
        state
            .put_ibc_asset(asset_1.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");
        state
            .put_ibc_asset(asset_2.clone().unwrap_trace_prefixed())
            .expect("should be able to call other trait method on state object");

        // create needed variables
        let address = astria_address(&[42u8; 20]);
        let amount_expected_0 = 1u128;
        let amount_expected_1 = 2u128;
        let amount_expected_2 = 3u128;

        // add balances to the account
        state
            .put_account_balance(&address, &asset_0, amount_expected_0)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(&address, &asset_1, amount_expected_1)
            .expect("putting an account balance should not fail");
        state
            .put_account_balance(&address, &asset_2, amount_expected_2)
            .expect("putting an account balance should not fail");

        let balances = get_account_balances(state, &address).await.unwrap();

        assert_eq!(
            balances.get(&asset_0.to_ibc_prefixed()).unwrap(),
            &amount_expected_0,
            "returned value for ibc asset_0 does not match"
        );
        assert_eq!(
            balances
                .get(&asset_1.unwrap_trace_prefixed().to_ibc_prefixed())
                .unwrap(),
            &amount_expected_1,
            "returned value for ibc asset_1 does not match"
        );
        assert_eq!(
            balances
                .get(&asset_2.unwrap_trace_prefixed().to_ibc_prefixed())
                .unwrap(),
            &amount_expected_2,
            "returned value for ibc asset_2 does not match"
        );
        assert_eq!(balances.len(), 3, "should only return existing values");
    }
}
