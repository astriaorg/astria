use std::fmt::{
    self,
    Debug,
    Formatter,
};

use astria_core::primitive::v1::{
    asset::IbcPrefixed,
    ADDRESS_LEN,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ensure,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use penumbra_ibc::{
    IbcRelay,
    IbcRelayWithHandlers,
};
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::ibc::{
    host_interface::AstriaHost,
    ics20_transfer::Ics20Transfer,
    StateReadExt as _,
};

pub(crate) struct CheckedIbcRelay {
    action: IbcRelay,
    action_with_handlers: IbcRelayWithHandlers<Ics20Transfer, AstriaHost>,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedIbcRelay {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: IbcRelay,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        let action_with_handlers = action.clone().with_handler::<Ics20Transfer, AstriaHost>();

        // Run immutable checks.
        action_with_handlers
            .check_stateless(())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("stateless checks failed for ibc action")?;

        let checked_action = Self {
            action,
            action_with_handlers,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        ensure!(
            state
                .is_ibc_relayer(&self.tx_signer)
                .await
                .wrap_err("failed to check if address is IBC relayer")?,
            "transaction signer not authorized to execute IBC actions"
        );
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;
        self.action_with_handlers
            .check_and_execute(state)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed executing ibc action")
    }

    pub(super) fn action(&self) -> &IbcRelay {
        &self.action
    }
}

impl AssetTransfer for CheckedIbcRelay {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}
impl Debug for CheckedIbcRelay {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CheckedIbcRelay")
            .field("action", self.action_with_handlers.action())
            .field("tx_signer", &self.tx_signer)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::ADDRESS_LENGTH,
        protocol::transaction::v1::action::IbcRelayerChange,
    };
    use ibc_proto::google::protobuf::Any;
    use ibc_types::core::client::{
        msgs::MsgCreateClient,
        ClientId,
    };
    use penumbra_ibc::{
        component::ClientStateReadExt as _,
        IbcRelay,
    };

    use super::*;
    use crate::{
        app::StateWriteExt as _,
        checked_actions::CheckedIbcRelayerChange,
        test_utils::{
            assert_error_contains,
            dummy_ibc_client_state,
            dummy_ibc_relay,
            Fixture,
            IBC_SUDO_ADDRESS,
            IBC_SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_stateless_checks_fail() {
        let fixture = Fixture::default_initialized().await;

        let action = IbcRelay::CreateClient(MsgCreateClient {
            client_state: Any::default(),
            consensus_state: Any::default(),
            signer: String::new(),
        });
        let err = fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "stateless checks failed for ibc action");
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_not_authorized() {
        let fixture = Fixture::default_initialized().await;

        let action = dummy_ibc_relay();
        let tx_signer = [2_u8; ADDRESS_LENGTH];
        assert_ne!(*IBC_SUDO_ADDRESS_BYTES, tx_signer);
        let err = fixture
            .new_checked_action(action, tx_signer)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to execute IBC actions",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_not_authorized() {
        // `IBC_SUDO_ADDRESS_BYTES` default initialized as the IBC sudo and relayer address.
        let mut fixture = Fixture::default_initialized().await;

        // Construct the checked action while the tx signer is recorded as the IBC relayer.
        let action = dummy_ibc_relay();
        let checked_action: CheckedIbcRelay = fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Remove the IBC relayer.
        let remove_relayer_action = IbcRelayerChange::Removal(*IBC_SUDO_ADDRESS);
        let checked_remove_relayer_action: CheckedIbcRelayerChange = fixture
            .new_checked_action(remove_relayer_action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_remove_relayer_action
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute the checked action now - should fail due to signer no longer being
        // authorized.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to execute IBC actions",
        );
    }

    #[tokio::test]
    async fn should_execute() {
        let mut fixture = Fixture::default_initialized().await;
        fixture.state_mut().put_block_height(1).unwrap();
        fixture.state_mut().put_revision_number(1).unwrap();
        let timestamp = tendermint::Time::from_unix_timestamp(1, 0).unwrap();
        fixture.state_mut().put_block_timestamp(timestamp).unwrap();

        let action = dummy_ibc_relay();
        let checked_action: CheckedIbcRelay = fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        let client_state = fixture
            .state()
            .get_client_state(&ClientId::default())
            .await
            .unwrap();
        assert_eq!(client_state, dummy_ibc_client_state(1));
    }
}
