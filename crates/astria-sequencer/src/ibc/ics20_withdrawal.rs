use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::{
        asset::Denom,
        Address,
    },
    protocol::transaction::v1alpha1::action,
    Protobuf as _,
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

use crate::{
    accounts::{
        AddressBytes,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    assets::StateWriteExt as _,
    bridge::StateReadExt as _,
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    state_ext::StateReadExt as _,
    transaction::StateReadExt as _,
};

fn withdrawal_to_unchecked_ibc_packet(
    withdrawal: &action::Ics20Withdrawal,
) -> IBCPacket<Unchecked> {
    let packet_data = withdrawal.to_fungible_token_packet_data();
    let serialized_packet_data =
        serde_json::to_vec(&packet_data).expect("can serialize FungibleTokenPacketData as JSON");

    IBCPacket::new(
        PortId::transfer(),
        withdrawal.source_channel().clone(),
        *withdrawal.timeout_height(),
        withdrawal.timeout_time(),
        serialized_packet_data,
    )
}

/// Establishes the withdrawal target.
///
/// The function returns the following addresses under the following conditions:
/// 1. `from` if `action.bridge_address` is unset and `from` is *not* a bridge account;
/// 2. `from` if `action.bridge_address` is unset and `from` is a bridge account and `from` is its
///    stored withdrawer address.
/// 3. `action.bridge_address` if `action.bridge_address` is set and a bridge account and `from` is
///    its stored withdrawer address.
async fn establish_withdrawal_target<S: StateRead>(
    action: &action::Ics20Withdrawal,
    state: &S,
    from: [u8; 20],
) -> Result<[u8; 20]> {
    if action.bridge_address.is_none()
        && !state
            .is_a_bridge_account(from)
            .await
            .context("failed to get bridge account rollup id")?
    {
        return Ok(from);
    }

    // if `action.bridge_address` is set, but it's not a valid bridge account,
    // the `get_bridge_account_withdrawer_address` step will fail.
    let bridge_address = action.bridge_address.map_or(from, Address::bytes);

    let Some(withdrawer) = state
        .get_bridge_account_withdrawer_address(bridge_address)
        .await
        .context("failed to get bridge withdrawer")?
    else {
        bail!("bridge address must have a withdrawer address set");
    };

    ensure!(
        withdrawer == from.address_bytes(),
        "sender does not match bridge withdrawer address; unauthorized"
    );

    Ok(bridge_address)
}

#[async_trait::async_trait]
impl ActionHandler for action::Ics20Withdrawal {
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.timeout_time() != 0, "timeout time must be non-zero",);

        // NOTE (from penumbra): we could validate the destination chain address as bech32 to
        // prevent mistyped addresses, but this would preclude sending to chains that don't
        // use bech32 addresses.
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_current_source()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        state
            .ensure_base_prefix(&self.return_address)
            .await
            .context("failed to verify that return address address has permitted base prefix")?;

        if let Some(bridge_address) = &self.bridge_address {
            state.ensure_base_prefix(bridge_address).await.context(
                "failed to verify that bridge address address has permitted base prefix",
            )?;
        }

        let withdrawal_target = establish_withdrawal_target(self, &state, from)
            .await
            .context("failed establishing which account to withdraw funds from")?;

        let fee = state
            .get_ics20_withdrawal_base_fee()
            .await
            .context("failed to get ics20 withdrawal base fee")?;

        let current_timestamp = state
            .get_block_timestamp()
            .await
            .context("failed to get block timestamp")?;
        let packet = {
            let packet = withdrawal_to_unchecked_ibc_packet(self);
            state
                .send_packet_check(packet, current_timestamp)
                .await
                .context("packet failed send check")?
        };

        state
            .get_and_increase_block_fees(self.fee_asset(), fee, Self::full_name())
            .await
            .context("failed to get and increase block fees")?;

        state
            .decrease_balance(withdrawal_target, self.denom(), self.amount())
            .await
            .context("failed to decrease sender or bridge balance")?;

        state
            .decrease_balance(from, self.fee_asset(), fee)
            .await
            .context("failed to subtract fee from sender balance")?;

        // if we're the source, move tokens to the escrow account,
        // otherwise the tokens are just burned
        if is_source(packet.source_port(), packet.source_channel(), self.denom()) {
            let channel_balance = state
                .get_ibc_channel_balance(self.source_channel(), self.denom())
                .await
                .context("failed to get channel balance")?;

            state
                .put_ibc_channel_balance(
                    self.source_channel(),
                    self.denom(),
                    channel_balance
                        .checked_add(self.amount())
                        .context("overflow when adding to channel balance")?,
                )
                .context("failed to update channel balance")?;
        }

        state.send_packet_execute(packet).await;
        Ok(())
    }
}

fn is_source(source_port: &PortId, source_channel: &ChannelId, asset: &Denom) -> bool {
    if let Denom::TracePrefixed(trace) = asset {
        !trace.starts_with_str(&format!("{source_port}/{source_channel}"))
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::RollupId;
    use cnidarium::StateDelta;
    use ibc_types::core::client::Height;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        bridge::StateWriteExt as _,
        test_utils::{
            assert_anyhow_error,
            astria_address,
            ASTRIA_PREFIX,
        },
    };

    #[tokio::test]
    async fn sender_is_withdrawal_target_if_bridge_is_not_set_and_sender_is_not_bridge() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let denom = "test".parse::<Denom>().unwrap();
        let from = [1u8; 20];
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: astria_address(&from),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: denom.clone(),
            memo: String::new(),
        };

        assert_eq!(
            establish_withdrawal_target(&action, &state, from)
                .await
                .unwrap(),
            from
        );
    }

    #[tokio::test]
    async fn sender_is_withdrawal_target_if_bridge_is_unset_but_sender_is_bridge() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        // sender is a bridge address, which is also the withdrawer, so it's ok
        let bridge_address = [1u8; 20];
        state.put_bridge_account_rollup_id(
            bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(bridge_address, bridge_address);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: astria_address(&bridge_address),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: denom.clone(),
            memo: String::new(),
        };

        assert_eq!(
            establish_withdrawal_target(&action, &state, bridge_address)
                .await
                .unwrap(),
            bridge_address,
        );
    }

    mod bridge_sender_is_rejected_because_it_is_not_a_withdrawer {
        use super::*;

        fn bridge_address() -> [u8; 20] {
            [1; 20]
        }

        fn denom() -> Denom {
            "test".parse().unwrap()
        }

        fn action() -> action::Ics20Withdrawal {
            action::Ics20Withdrawal {
                amount: 1,
                denom: denom(),
                bridge_address: None,
                destination_chain_address: "test".to_string(),
                return_address: astria_address(&[1; 20]),
                timeout_height: Height::new(1, 1).unwrap(),
                timeout_time: 1,
                source_channel: "channel-0".to_string().parse().unwrap(),
                fee_asset: denom(),
                memo: String::new(),
            }
        }

        async fn run_test(action: action::Ics20Withdrawal) {
            let storage = cnidarium::TempStorage::new().await.unwrap();
            let snapshot = storage.latest_snapshot();
            let mut state = StateDelta::new(snapshot);

            state.put_base_prefix(ASTRIA_PREFIX).unwrap();

            // withdraw is *not* the bridge address, Ics20Withdrawal must be sent by the withdrawer
            state.put_bridge_account_rollup_id(
                bridge_address(),
                &RollupId::from_unhashed_bytes("testrollupid"),
            );
            state.put_bridge_account_withdrawer_address(
                bridge_address(),
                astria_address(&[2u8; 20]),
            );

            assert_anyhow_error(
                &establish_withdrawal_target(&action, &state, bridge_address())
                    .await
                    .unwrap_err(),
                "sender does not match bridge withdrawer address; unauthorized",
            );
        }

        #[tokio::test]
        async fn bridge_unset() {
            let mut action = action();
            action.bridge_address = None;
            run_test(action).await;
        }

        #[tokio::test]
        async fn bridge_set() {
            let mut action = action();
            action.bridge_address = Some(astria_address(&bridge_address()));
            run_test(action).await;
        }
    }

    #[tokio::test]
    async fn bridge_sender_is_withdrawal_target() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        // sender the withdrawer address, so it's ok
        let bridge_address = [1u8; 20];
        let withdrawer_address = [2u8; 20];
        state.put_bridge_account_rollup_id(
            bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(bridge_address, withdrawer_address);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: Some(astria_address(&bridge_address)),
            destination_chain_address: "test".to_string(),
            return_address: astria_address(&bridge_address),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: denom.clone(),
            memo: String::new(),
        };

        assert_eq!(
            establish_withdrawal_target(&action, &state, withdrawer_address)
                .await
                .unwrap(),
            bridge_address,
        );
    }

    #[tokio::test]
    async fn bridge_is_rejected_as_withdrawal_target_because_it_has_no_withdrawer_address_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        // sender is not the withdrawer address, so must fail
        let not_bridge_address = [1u8; 20];

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: Some(astria_address(&not_bridge_address)),
            destination_chain_address: "test".to_string(),
            return_address: astria_address(&not_bridge_address),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: denom.clone(),
            memo: String::new(),
        };

        assert_anyhow_error(
            &establish_withdrawal_target(&action, &state, not_bridge_address)
                .await
                .unwrap_err(),
            "bridge address must have a withdrawer address set",
        );
    }
}
