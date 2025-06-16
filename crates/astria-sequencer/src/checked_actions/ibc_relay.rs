use std::fmt::{
    self,
    Debug,
    Formatter,
};

use astria_core::{
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    upgrades::v1::blackburn::{
        Blackburn,
        Ics20TransferActionChange,
    },
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
use ibc_proto::ibc::apps::transfer::v2::FungibleTokenPacketData;
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
use crate::{
    fees::StateReadExt as _,
    ibc::{
        host_interface::AstriaHost,
        ics20_transfer::{
            is_transfer_source_zone,
            parse_asset,
            Ics20Transfer,
        },
        StateReadExt as _,
    },
    upgrades::StateReadExt as _,
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

        // Only allow ics20 transfers of fee assets post-Blackburn upgrade
        if !use_pre_blackburn_ics20_transfer(&state)
            .await
            .wrap_err("failed to get blackburn upgrade status")?
        {
            if let IbcRelay::RecvPacket(msg_recv_packet) = &self.action {
                let packet_data: FungibleTokenPacketData =
                    serde_json::from_slice(&msg_recv_packet.packet.data)
                        .wrap_err("failed to deserialize fungible token packet data")?;
                let mut asset = parse_asset(&state, &packet_data.denom)
                    .await
                    .wrap_err_with(|| {
                        format!(
                            "failed reading asset `{}` from packet data",
                            packet_data.denom
                        )
                    })?;
                let is_source = is_transfer_source_zone(
                    &asset,
                    &msg_recv_packet.packet.port_on_a,
                    &msg_recv_packet.packet.chan_on_a,
                );
                if is_source {
                    asset.pop_leading_port_and_channel();
                } else {
                    asset = format!(
                        "{destination_port}/{destination_channel}/{asset}",
                        destination_port = &msg_recv_packet.packet.port_on_b,
                        destination_channel = &msg_recv_packet.packet.chan_on_b,
                    )
                    .parse()
                    .wrap_err("failed to parse destination asset from packet data")?;
                }
                ensure!(
                    state
                        .is_allowed_fee_asset(&asset)
                        .await
                        .wrap_err("failed to check if asset is allowed fee asset")?,
                    "denom `{}` is not an allowed asset for ics20 transfer post blackburn \
                     upgrade; only allowed fee assets can be transferred using ics20 transfers",
                    asset
                );
            }
        }
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

async fn use_pre_blackburn_ics20_transfer<S: StateRead>(state: &S) -> Result<bool> {
    Ok(state
        .get_upgrade_change_info(&Blackburn::NAME, &Ics20TransferActionChange::NAME)
        .await
        .wrap_err(
            "failed to read upgrade change info for ics20 transfer action change from storage",
        )?
        .is_none())
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::ADDRESS_LENGTH,
        primitive::v1::asset::Denom,
        protocol::transaction::v1::action::{
            FeeAssetChange,
            IbcRelayerChange,
        },
    };
    use ibc_proto::google::protobuf::Any;
    use ibc_types::{
        core::{
            channel::{
                msgs::MsgRecvPacket,
                packet::Sequence,
                ChannelId,
                Packet,
                PortId,
                TimeoutHeight,
            },
            client::{
                msgs::MsgCreateClient,
                ClientId,
                Height,
            },
            commitment::MerkleProof,
        },
        timestamp::Timestamp,
    };
    use penumbra_ibc::{
        component::ClientStateReadExt as _,
        IbcRelay,
    };

    use super::*;
    use crate::{
        app::StateWriteExt as _,
        checked_actions::{
            CheckedFeeAssetChange,
            CheckedIbcRelayerChange,
        },
        test_utils::{
            assert_error_contains,
            dummy_ibc_client_state,
            dummy_ibc_relay,
            Fixture,
            IBC_SUDO_ADDRESS,
            IBC_SUDO_ADDRESS_BYTES,
            SUDO_ADDRESS_BYTES,
        },
    };

    const PORT_A: &str = "port-a";
    const CHANNEL_A: &str = "channel-a";

    fn dummy_ics20_transfer(denom: String) -> IbcRelay {
        let packet_data = FungibleTokenPacketData {
            denom,
            amount: "1000".to_string(),
            sender: "sender-address".to_string(),
            receiver: "receiver-address".to_string(),
            memo: String::new(),
        };

        let packet = Packet {
            sequence: Sequence::default(),
            port_on_a: PortId(PORT_A.to_string()),
            chan_on_a: ChannelId(CHANNEL_A.to_string()),
            port_on_b: PortId("port-b".to_string()),
            chan_on_b: ChannelId("channel-b".to_string()),
            data: serde_json::to_vec(&packet_data).unwrap(),
            timeout_height_on_b: TimeoutHeight::default(),
            timeout_timestamp_on_b: Timestamp::default(),
        };

        IbcRelay::RecvPacket(MsgRecvPacket {
            packet,
            proof_commitment_on_a: MerkleProof {
                proofs: vec![],
            },
            proof_height_on_a: Height::new(1, 1).unwrap(),
            signer: "signer-address".to_string(),
        })
    }

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
    async fn pre_blackburn_ics20_transfer_construction_succeeds_if_asset_not_allowed() {
        // Only apply aspen upgrade
        let mut fixture = Fixture::uninitialized(None).await;
        fixture.chain_initializer().init().await;

        let denom = "utia".to_string();

        // Construct IbcRelay action with ICS20 transfer.
        let action = dummy_ics20_transfer(denom.clone());

        // Ensure ICS20 transfer action asset is not an allowed fee asset.
        assert!(!fixture
            .state()
            .is_allowed_fee_asset(&(denom.parse::<Denom>().unwrap()))
            .await
            .unwrap());

        fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn post_blackburn_ics20_transfer_should_fail_construction_if_asset_not_allowed() {
        // The default initializer will run until Blackburn upgrade is applied.
        let fixture = Fixture::default_initialized().await;

        let denom = "utia".parse::<Denom>().unwrap();
        let sequencer_originating_denom = format!("{PORT_A}/{CHANNEL_A}/{denom}");

        // Construct IbcRelay action with ICS20 transfer.
        let action = dummy_ics20_transfer(sequencer_originating_denom);

        // Ensure ICS20 transfer action asset is not an allowed fee asset.
        assert!(!fixture
            .state()
            .is_allowed_fee_asset(&(denom.clone()))
            .await
            .unwrap());

        let err = fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "denom `{denom}` is not an allowed asset for ics20 transfer post blackburn \
                 upgrade; only allowed fee assets can be transferred using ics20 transfers",
            ),
        );
    }

    #[tokio::test]
    async fn post_blackburn_ics20_transfer_should_fail_execution_if_asset_not_allowed() {
        let mut fixture = Fixture::default_initialized().await;

        let denom = "utia".parse::<Denom>().unwrap();

        // Add asset to allowed fee assets (cannot use existing fee asset since we cannot remove
        // it).
        let add_fee_asset_action = FeeAssetChange::Addition(denom.clone());
        let checked_add_fee_asset_action: CheckedFeeAssetChange = fixture
            .new_checked_action(add_fee_asset_action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_add_fee_asset_action
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Port and channel prefix will be stripped since they match the origin chain
        let sequencer_originating_denom = format!("{PORT_A}/{CHANNEL_A}/{denom}");

        assert!(fixture.state().is_allowed_fee_asset(&denom).await.unwrap());

        // Construct IbcRelay action with ICS20 transfer.
        let action = dummy_ics20_transfer(sequencer_originating_denom);
        let checked_ics20_transfer_action: CheckedIbcRelay = fixture
            .new_checked_action(action, *IBC_SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Remove the asset from list of allowed fee assets.
        let remove_fee_asset_action = FeeAssetChange::Removal(denom.clone());
        let checked_remove_fee_asset_action: CheckedFeeAssetChange = fixture
            .new_checked_action(remove_fee_asset_action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_remove_fee_asset_action
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try execute - should fail.
        let err = checked_ics20_transfer_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "denom `{denom}` is not an allowed asset for ics20 transfer post blackburn \
                 upgrade; only allowed fee assets can be transferred using ics20 transfers",
            ),
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
