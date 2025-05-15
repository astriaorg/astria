use astria_core::{
    primitive::v1::{
        asset::Denom,
        Address,
        RollupId,
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::InitBridgeAccount,
};

use super::{
    astria_address,
    Fixture,
    SUDO_ADDRESS,
};
use crate::{
    accounts::{
        AddressBytes,
        StateWriteExt as _,
    },
    fees::StateReadExt as _,
    test_utils::nria,
};

/// A helper to simplify initializing a new bridge account in the `App`'s state delta of a given
/// [`Fixture`].
///
/// An instance can be constructed via [`Fixture::bridge_initializer`].
///
/// By default, the following values are used:
///   * `rollup_id`: `[1; 32]`
///   * `asset`: nria
///   * `sudo_address`: `SUDO_ADDRESS`
///   * `withdrawer_address`: `SUDO_ADDRESS`
pub(crate) struct BridgeInitializer<'a> {
    fixture: &'a mut Fixture,
    bridge_address: Address,
    action: InitBridgeAccount,
}

impl<'a> BridgeInitializer<'a> {
    pub(super) fn new(fixture: &'a mut Fixture, bridge_address: Address) -> Self {
        Self {
            fixture,
            bridge_address,
            action: InitBridgeAccount {
                rollup_id: RollupId::new([1; 32]),
                asset: nria().into(),
                fee_asset: nria().into(),
                sudo_address: Some(*SUDO_ADDRESS),
                withdrawer_address: Some(*SUDO_ADDRESS),
            },
        }
    }

    pub(crate) fn with_asset<T: Into<Denom>>(mut self, asset: T) -> Self {
        self.action.asset = asset.into();
        self
    }

    pub(crate) fn with_rollup_id(mut self, rollup_id: RollupId) -> Self {
        self.action.rollup_id = rollup_id;
        self
    }

    pub(crate) fn with_withdrawer_address(mut self, withdrawer_address: [u8; ADDRESS_LEN]) -> Self {
        self.action.withdrawer_address = Some(astria_address(&withdrawer_address));
        self
    }

    pub(crate) async fn init(self) {
        let Self {
            fixture,
            bridge_address,
            action,
        } = self;

        let mut state_delta = fixture.app.new_state_delta();
        let checked_action = fixture
            .new_checked_action(action, *bridge_address.address_bytes())
            .await
            .unwrap();
        // Add balance to cover the execution costs.
        if let Some(fees) = state_delta.get_fees::<InitBridgeAccount>().await.unwrap() {
            state_delta
                .put_account_balance(&bridge_address, &nria(), fees.base())
                .unwrap();
        }
        checked_action
            .pay_fees_and_execute(
                &mut state_delta,
                bridge_address.address_bytes(),
                &TransactionId::new([1; 32]),
                0,
            )
            .await
            .unwrap();

        fixture
            .app
            .apply_and_commit(state_delta, fixture.storage())
            .await;
    }
}
