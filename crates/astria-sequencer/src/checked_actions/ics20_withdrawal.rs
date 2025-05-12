use std::fmt::{
    self,
    Debug,
    Formatter,
};

use astria_core::{
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
        },
        Address,
        Bech32,
        ADDRESS_LEN,
    },
    protocol::{
        memos::v1::Ics20WithdrawalFromRollup,
        transaction::v1::action::Ics20Withdrawal,
    },
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        ensure,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use ibc_types::core::channel::{
    ChannelId,
    PortId,
};
use penumbra_ibc::component::packet::{
    IBCPacket,
    SendPacketRead as _,
    SendPacketWrite as _,
    Unchecked,
};
use penumbra_proto::core::component::ibc::v1::FungibleTokenPacketData;
use tracing::{
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    app::StateReadExt as _,
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

pub(crate) struct CheckedIcs20Withdrawal {
    action: Ics20Withdrawal,
    withdrawal_address: [u8; ADDRESS_LEN],
    bridge_address_and_rollup_withdrawal: Option<(Address, Ics20WithdrawalFromRollup)>,
    ibc_packet: IBCPacket<Unchecked>,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedIcs20Withdrawal {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: Ics20Withdrawal,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        state
            .ensure_base_prefix(&action.return_address)
            .await
            .wrap_err("failed to verify that return address has permitted base prefix")?;

        ensure!(action.timeout_time != 0, "timeout time must be non-zero");
        ensure!(action.amount > 0, "amount must be greater than zero");
        let withdrawal_address = action
            .bridge_address
            .as_ref()
            .map_or(tx_signer, |address| address.bytes());
        let bridge_address_and_rollup_withdrawal = if let Some(bridge_address) =
            action.bridge_address
        {
            state
                .ensure_base_prefix(&bridge_address)
                .await
                .wrap_err("bridge address has an unsupported prefix")?;
            let parsed_bridge_memo: Ics20WithdrawalFromRollup = serde_json::from_str(&action.memo)
                .wrap_err("failed to parse memo for outgoing IBC bridge withdrawal")?;
            ensure!(
                !parsed_bridge_memo.rollup_return_address.is_empty(),
                "rollup return address must be non-empty",
            );
            ensure!(
                parsed_bridge_memo.rollup_return_address.len() <= 256,
                "rollup return address must be no more than 256 bytes",
            );
            ensure!(
                !parsed_bridge_memo.rollup_withdrawal_event_id.is_empty(),
                "rollup withdrawal event id must be non-empty",
            );
            ensure!(
                parsed_bridge_memo.rollup_withdrawal_event_id.len() <= 256,
                "rollup withdrawal event id must be no more than 256 bytes",
            );
            ensure!(
                parsed_bridge_memo.rollup_block_number != 0,
                "rollup block number must be non-zero",
            );
            Some((bridge_address, parsed_bridge_memo))
        } else {
            None
        };

        let ibc_packet = create_ibc_packet_from_withdrawal(action.clone(), &state).await?;
        let tx_signer = TransactionSignerAddressBytes::from(tx_signer);

        let checked_action = Self {
            action,
            withdrawal_address,
            bridge_address_and_rollup_withdrawal,
            ibc_packet,
            tx_signer,
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        if let Some((bridge_address, rollup_withdrawal)) =
            &self.bridge_address_and_rollup_withdrawal
        {
            let Some(withdrawer) = state
                .get_bridge_account_withdrawer_address(bridge_address)
                .await
                .wrap_err("failed to read bridge withdrawer address from storage")?
            else {
                bail!("bridge account does not have an associated withdrawer address in storage");
            };

            ensure!(
                &withdrawer == self.tx_signer.as_bytes(),
                "transaction signer not authorized to perform ics20 bridge withdrawal"
            );

            if let Some(existing_block_num) = state
                .get_withdrawal_event_rollup_block_number(
                    &self.withdrawal_address,
                    &rollup_withdrawal.rollup_withdrawal_event_id,
                )
                .await
                .wrap_err(
                    "failed to read bridge account withdrawal event block height from storage",
                )?
            {
                bail!(
                    "withdrawal event ID `{}` was already executed (rollup block number \
                     {existing_block_num})",
                    rollup_withdrawal.rollup_withdrawal_event_id
                );
            }
        } else if state
            .is_a_bridge_account(&self.tx_signer)
            .await
            .wrap_err("failed to establish whether the signer is a bridge account")?
        {
            bail!("signer cannot be a bridge address if bridge address is not set");
        }

        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;
        if let Some((_bridge_address, rollup_withdrawal)) =
            &self.bridge_address_and_rollup_withdrawal
        {
            state
                .put_withdrawal_event_rollup_block_number(
                    &self.withdrawal_address,
                    &rollup_withdrawal.rollup_withdrawal_event_id,
                    rollup_withdrawal.rollup_block_number,
                )
                .wrap_err("failed to write withdrawal event block to storage")?;
        }

        let current_timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to read block timestamp from storage")?;
        // `IBCPacket<Unchecked>` doesn't implement `Clone` - manually clone it.
        let unchecked_packet = IBCPacket::new(
            self.ibc_packet.source_port().clone(),
            self.ibc_packet.source_channel().clone(),
            *self.ibc_packet.timeout_height(),
            self.ibc_packet.timeout_timestamp(),
            self.ibc_packet.data().to_vec(),
        );
        let checked_packet = state
            .send_packet_check(unchecked_packet, current_timestamp)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("ibc packet failed send check")?;

        state
            .decrease_balance(
                &self.withdrawal_address,
                &self.action.denom,
                self.action.amount,
            )
            .await
            .wrap_err("failed to decrease sender or bridge balance")?;

        // If we're the source, move tokens to the escrow account, otherwise the tokens are just
        // burned.
        if is_source(
            checked_packet.source_port(),
            checked_packet.source_channel(),
            &self.action.denom,
        ) {
            let channel_balance = state
                .get_ibc_channel_balance(self.ibc_packet.source_channel(), &self.action.denom)
                .await
                .wrap_err("failed to read channel balance from storage")?;

            state
                .put_ibc_channel_balance(
                    self.ibc_packet.source_channel(),
                    &self.action.denom,
                    channel_balance
                        .checked_add(self.action.amount)
                        .ok_or_eyre("overflow when adding to channel balance")?,
                )
                .wrap_err("failed to write channel balance to storage")?;
        }

        state.send_packet_execute(checked_packet).await;
        Ok(())
    }

    pub(super) fn action(&self) -> &Ics20Withdrawal {
        &self.action
    }
}

impl AssetTransfer for CheckedIcs20Withdrawal {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        Some((self.action.denom.to_ibc_prefixed(), self.action.amount))
    }
}

impl Debug for CheckedIcs20Withdrawal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CheckedIcs20Withdrawal")
            .field("action", &self.action)
            .field("withdrawal_address", &self.withdrawal_address)
            .field(
                "bridge_address_and_rollup_withdrawal",
                &self.bridge_address_and_rollup_withdrawal,
            )
            .field("ibc_packet.source_port", self.ibc_packet.source_port())
            .field(
                "ibc_packet.source_channel",
                self.ibc_packet.source_channel(),
            )
            .field(
                "ibc_packet.timeout_height",
                self.ibc_packet.timeout_height(),
            )
            .field(
                "ibc_packet.timeout_timestamp",
                &self.ibc_packet.timeout_timestamp(),
            )
            .field(
                "ibc_packet.data",
                &String::from_utf8_lossy(self.ibc_packet.data()),
            )
            .field("tx_signer", &self.tx_signer)
            .finish()
    }
}

async fn create_ibc_packet_from_withdrawal<S: StateRead>(
    withdrawal: Ics20Withdrawal,
    state: S,
) -> Result<IBCPacket<Unchecked>> {
    let sender = if withdrawal.use_compat_address {
        let ibc_compat_prefix = state.get_ibc_compat_prefix().await.wrap_err(
            "need to construct bech32 compatible address for IBC communication but failed reading \
             required prefix from state",
        )?;
        withdrawal
            .return_address
            .to_prefix(&ibc_compat_prefix)
            .wrap_err("failed to convert the address to the bech32 compatible prefix")?
            .to_format::<Bech32>()
            .to_string()
    } else {
        withdrawal.return_address.to_string()
    };
    let packet = FungibleTokenPacketData {
        amount: withdrawal.amount.to_string(),
        denom: withdrawal.denom.to_string(),
        sender,
        receiver: withdrawal.destination_chain_address,
        memo: withdrawal.memo,
    };

    let serialized_packet_data = serde_json::to_vec(&packet)
        .wrap_err("failed to serialize fungible token packet as JSON")?;

    Ok(IBCPacket::new(
        PortId::transfer(),
        withdrawal.source_channel,
        withdrawal.timeout_height,
        withdrawal.timeout_time,
        serialized_packet_data,
    ))
}

fn is_source(source_port: &PortId, source_channel: &ChannelId, asset: &Denom) -> bool {
    if let Denom::TracePrefixed(trace) = asset {
        !trace.has_leading_port(source_port) || !trace.has_leading_channel(source_channel)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use astria_core::{
        primitive::v1::RollupId,
        protocol::transaction::v1::action::{
            BridgeSudoChange,
            InitBridgeAccount,
        },
    };

    use super::{
        super::test_utils::address_with_prefix,
        *,
    };
    use crate::{
        checked_actions::{
            CheckedBridgeSudoChange,
            CheckedInitBridgeAccount,
        },
        test_utils::{
            assert_error_contains,
            astria_address,
            Fixture,
            Ics20WithdrawalBuilder,
            ASTRIA_PREFIX,
            SUDO_ADDRESS,
            SUDO_ADDRESS_BYTES,
        },
    };

    #[tokio::test]
    async fn should_fail_construction_if_return_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = Ics20WithdrawalBuilder::new()
            .with_return_address(address_with_prefix([50; ADDRESS_LEN], prefix))
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_timeout_time_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new().with_timeout_time(0).build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "timeout time must be non-zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_amount_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new().with_amount(0).build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "amount must be greater than zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_address_not_base_prefixed() {
        let fixture = Fixture::default_initialized().await;

        let prefix = "different_prefix";
        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(address_with_prefix([50; ADDRESS_LEN], prefix))
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!("address has prefix `{prefix}` but only `{ASTRIA_PREFIX}` is permitted"),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_memo_fails_to_parse() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "failed to parse memo for outgoing IBC bridge withdrawal",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_return_address_is_empty() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .with_rollup_return_address("")
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup return address must be non-empty");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_return_address_is_too_long() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .with_rollup_return_address(iter::repeat_n('a', 257).collect::<String>())
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup return address must be no more than 256 bytes");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_withdrawal_event_id_is_empty() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .with_rollup_withdrawal_event_id("")
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup withdrawal event id must be non-empty");
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_withdrawal_event_id_is_too_long() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .with_rollup_withdrawal_event_id(iter::repeat_n('a', 257).collect::<String>())
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "rollup withdrawal event id must be no more than 256 bytes",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_rollup_block_number_is_zero() {
        let fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(astria_address(&[2; ADDRESS_LEN]))
            .with_rollup_block_number(0)
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(&err, "rollup block number must be non-zero");
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_account_withdrawer_is_not_tx_signer() {
        let mut fixture = Fixture::default_initialized().await;
        let bridge_address = astria_address(&[2; ADDRESS_LEN]);
        let withdrawer_address = astria_address(&[3; ADDRESS_LEN]);
        fixture
            .bridge_initializer(bridge_address)
            .with_withdrawer_address(withdrawer_address.bytes())
            .init()
            .await;
        assert_ne!(withdrawer_address.bytes(), *SUDO_ADDRESS_BYTES);

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(bridge_address)
            .with_default_rollup_withdrawal()
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "transaction signer not authorized to perform ics20 bridge withdrawal",
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_account_withdrawal_event_already_processed() {
        let mut fixture = Fixture::default_initialized().await;
        let bridge_address = astria_address(&[2; ADDRESS_LEN]);
        fixture.bridge_initializer(bridge_address).init().await;
        let event_id = "event-1".to_string();
        let block_number = 2;
        fixture
            .state_mut()
            .put_withdrawal_event_rollup_block_number(&bridge_address, &event_id, block_number)
            .unwrap();

        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(bridge_address)
            .with_rollup_withdrawal_event_id(&event_id)
            .build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            &format!(
                "withdrawal event ID `{event_id}` was already executed (rollup block number \
                 {block_number})"
            ),
        );
    }

    #[tokio::test]
    async fn should_fail_construction_if_bridge_account_unset_and_tx_signer_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;
        fixture.bridge_initializer(*SUDO_ADDRESS).init().await;

        let action = Ics20WithdrawalBuilder::new().build();
        let err = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap_err();

        assert_error_contains(
            &err,
            "signer cannot be a bridge address if bridge address is not set",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_bridge_account_withdrawer_is_not_tx_signer() {
        let mut fixture = Fixture::default_initialized().await;

        // Store `SUDO_ADDRESS` as the bridge account sudo and withdrawer address.
        let bridge_address = astria_address(&[2; ADDRESS_LEN]);
        fixture.bridge_initializer(bridge_address).init().await;

        // Construct the checked ICS20 withdrawal action while the withdrawal address is still the
        // tx signer so construction succeeds.
        let action = Ics20WithdrawalBuilder::new()
            .with_bridge_address(bridge_address)
            .with_default_rollup_withdrawal()
            .build();
        let checked_action: CheckedIcs20Withdrawal = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Change the bridge account withdrawer address to one different from the tx signer address.
        let new_withdrawer_address = astria_address(&[3; ADDRESS_LEN]);
        assert_ne!(new_withdrawer_address.bytes(), *SUDO_ADDRESS_BYTES);
        let bridge_sudo_change = BridgeSudoChange {
            bridge_address,
            new_sudo_address: None,
            new_withdrawer_address: Some(new_withdrawer_address),
            fee_asset: "test".parse().unwrap(),
        };
        let checked_bridge_sudo_change: CheckedBridgeSudoChange = fixture
            .new_checked_action(bridge_sudo_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_bridge_sudo_change
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Try to execute checked ICS20 withdrawal action now - should fail due to signer no longer
        // being authorized.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to perform ics20 bridge withdrawal",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_bridge_account_unset_and_tx_signer_is_bridge_account() {
        let mut fixture = Fixture::default_initialized().await;

        let action = Ics20WithdrawalBuilder::new().build();
        let checked_action: CheckedIcs20Withdrawal = fixture
            .new_checked_action(action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Initialize the signer's account as a bridge account.
        let init_bridge_account = InitBridgeAccount {
            rollup_id: RollupId::new([1; 32]),
            asset: "test".parse().unwrap(),
            fee_asset: "test".parse().unwrap(),
            sudo_address: Some(*SUDO_ADDRESS),
            withdrawer_address: Some(*SUDO_ADDRESS),
        };
        let checked_init_bridge_account: CheckedInitBridgeAccount = fixture
            .new_checked_action(init_bridge_account, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_init_bridge_account
            .execute(fixture.state_mut())
            .await
            .unwrap();

        // Should now fail to execute as the withdrawal action does not have the bridge account set.
        let err = checked_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "signer cannot be a bridge address if bridge address is not set",
        );
    }
}
