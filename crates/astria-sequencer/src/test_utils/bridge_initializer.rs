use astria_core::primitive::v1::{
    asset::Denom,
    Address,
    RollupId,
    ADDRESS_LEN,
};

use super::{
    Fixture,
    SUDO_ADDRESS,
};
use crate::{
    accounts::AddressBytes,
    bridge::StateWriteExt as _,
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
    rollup_id: Option<RollupId>,
    asset: Denom,
    sudo_address: [u8; ADDRESS_LEN],
    withdrawer_address: Option<[u8; ADDRESS_LEN]>,
}

impl<'a> BridgeInitializer<'a> {
    pub(super) fn new(fixture: &'a mut Fixture, bridge_address: Address) -> Self {
        Self {
            fixture,
            bridge_address,
            rollup_id: Some(RollupId::new([1; 32])),
            asset: nria().into(),
            sudo_address: *SUDO_ADDRESS.address_bytes(),
            withdrawer_address: Some(*SUDO_ADDRESS.address_bytes()),
        }
    }

    pub(crate) fn with_asset<T: Into<Denom>>(mut self, asset: T) -> Self {
        self.asset = asset.into();
        self
    }

    pub(crate) fn with_rollup_id(mut self, rollup_id: RollupId) -> Self {
        self.rollup_id = Some(rollup_id);
        self
    }

    pub(crate) fn with_no_rollup_id(mut self) -> Self {
        self.rollup_id = None;
        self
    }

    pub(crate) fn with_withdrawer_address(mut self, withdrawer_address: [u8; ADDRESS_LEN]) -> Self {
        self.withdrawer_address = Some(withdrawer_address);
        self
    }

    pub(crate) fn with_no_withdrawer_address(mut self) -> Self {
        self.withdrawer_address = None;
        self
    }

    pub(crate) async fn init(self) {
        let Self {
            fixture,
            bridge_address,
            rollup_id,
            asset,
            sudo_address,
            withdrawer_address,
        } = self;

        let mut state_delta = fixture.app.new_state_delta();
        if let Some(rollup_id) = rollup_id {
            state_delta
                .put_bridge_account_rollup_id(&bridge_address, rollup_id)
                .unwrap();
        }
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, &asset)
            .unwrap();
        state_delta
            .put_bridge_account_sudo_address(&bridge_address, sudo_address)
            .unwrap();
        if let Some(withdrawer_address) = withdrawer_address {
            state_delta
                .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
                .unwrap();
        }
        fixture
            .app
            .apply_and_commit(state_delta, fixture.storage())
            .await;
    }
}
