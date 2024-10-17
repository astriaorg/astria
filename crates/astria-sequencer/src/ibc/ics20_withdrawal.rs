use astria_core::{
    primitive::v1::{
        asset::Denom,
        Address,
        Bech32,
    },
    protocol::{
        memos::v1::Ics20WithdrawalFromRollup,
        transaction::v1::action,
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

use crate::{
    accounts::{
        AddressBytes as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::{
        ActionHandler,
        StateReadExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

async fn create_ibc_packet_from_withdrawal<S: StateRead>(
    withdrawal: &action::Ics20Withdrawal,
    state: S,
) -> Result<IBCPacket<Unchecked>> {
    let sender = if withdrawal.use_compat_address {
        let ibc_compat_prefix = state.get_ibc_compat_prefix().await.context(
            "need to construct bech32 compatible address for IBC communication but failed reading \
             required prefix from state",
        )?;
        withdrawal
            .return_address()
            .to_prefix(&ibc_compat_prefix)
            .context("failed to convert the address to the bech32 compatible prefix")?
            .to_format::<Bech32>()
            .to_string()
    } else {
        withdrawal.return_address.to_string()
    };
    let packet = FungibleTokenPacketData {
        amount: withdrawal.amount.to_string(),
        denom: withdrawal.denom.to_string(),
        sender,
        receiver: withdrawal.destination_chain_address.clone(),
        memo: withdrawal.memo.clone(),
    };

    let serialized_packet_data =
        serde_json::to_vec(&packet).context("failed to serialize fungible token packet as JSON")?;

    Ok(IBCPacket::new(
        PortId::transfer(),
        withdrawal.source_channel().clone(),
        *withdrawal.timeout_height(),
        withdrawal.timeout_time(),
        serialized_packet_data,
    ))
}

/// Establishes the withdrawal target.
///
/// The function returns the following addresses under the following conditions:
/// 1. `action.bridge_address` if `action.bridge_address` is set and `from` is its stored withdrawer
///    address.
/// 2. `from` if `action.bridge_address` is unset and `from` is *not* a bridge account.
///
/// Errors if:
/// 1. Errors reading from DB
/// 2. `action.bridge_address` is set, but `from` is not the withdrawer address.
/// 3. `action.bridge_address` is unset, but `from` is a bridge account.
async fn establish_withdrawal_target<'a, S: StateRead>(
    action: &'a action::Ics20Withdrawal,
    state: &S,
    from: &'a [u8; 20],
) -> Result<&'a [u8; 20]> {
    // If the bridge address is set, the withdrawer on that address must match
    // the from address.
    if let Some(bridge_address) = &action.bridge_address {
        let Some(withdrawer) = state
            .get_bridge_account_withdrawer_address(bridge_address)
            .await
            .wrap_err("failed to get bridge withdrawer")?
        else {
            bail!("bridge address must have a withdrawer address set");
        };

        ensure!(
            &withdrawer == from.address_bytes(),
            "sender does not match bridge withdrawer address; unauthorized"
        );

        return Ok(bridge_address.as_bytes());
    }

    // If the bridge address is not set, the sender must not be a bridge account.
    if state
        .is_a_bridge_account(from)
        .await
        .context("failed to establish whether the sender is a bridge account")?
    {
        bail!("sender cannot be a bridge address if bridge address is not set");
    }

    Ok(from)
}

#[async_trait::async_trait]
impl ActionHandler for action::Ics20Withdrawal {
    // TODO(https://github.com/astriaorg/astria/issues/1430): move checks to the `Ics20Withdrawal` parsing.
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.timeout_time() != 0, "timeout time must be non-zero",);
        ensure!(self.amount() > 0, "amount must be greater than zero",);
        if self.bridge_address.is_some() {
            let parsed_bridge_memo: Ics20WithdrawalFromRollup = serde_json::from_str(&self.memo)
                .context("failed to parse memo for ICS bound bridge withdrawal")?;

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
        }

        // NOTE (from penumbra): we could validate the destination chain address as bech32 to
        // prevent mistyped addresses, but this would preclude sending to chains that don't
        // use bech32 addresses.
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        state
            .ensure_base_prefix(&self.return_address)
            .await
            .wrap_err("failed to verify that return address address has permitted base prefix")?;

        if let Some(bridge_address) = &self.bridge_address {
            state.ensure_base_prefix(bridge_address).await.wrap_err(
                "failed to verify that bridge address address has permitted base prefix",
            )?;
            let parsed_bridge_memo: Ics20WithdrawalFromRollup = serde_json::from_str(&self.memo)
                .context("failed to parse memo for ICS bound bridge withdrawal")?;

            state
                .check_and_set_withdrawal_event_block_for_bridge_account(
                    self.bridge_address
                        .as_ref()
                        .map_or(&from, Address::as_bytes),
                    &parsed_bridge_memo.rollup_withdrawal_event_id,
                    parsed_bridge_memo.rollup_block_number,
                )
                .await
                .context("withdrawal event already processed")?;
        }

        let withdrawal_target = establish_withdrawal_target(self, &state, &from)
            .await
            .wrap_err("failed establishing which account to withdraw funds from")?;

        let current_timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get block timestamp")?;
        let packet = {
            let packet = create_ibc_packet_from_withdrawal(self, &state)
                .await
                .context("failed converting the withdrawal action into IBC packet")?;
            state
                .send_packet_check(packet, current_timestamp)
                .await
                .map_err(anyhow_to_eyre)
                .wrap_err("packet failed send check")?
        };

        state
            .decrease_balance(withdrawal_target, self.denom(), self.amount())
            .await
            .wrap_err("failed to decrease sender or bridge balance")?;

        // if we're the source, move tokens to the escrow account,
        // otherwise the tokens are just burned
        if is_source(packet.source_port(), packet.source_channel(), self.denom()) {
            let channel_balance = state
                .get_ibc_channel_balance(self.source_channel(), self.denom())
                .await
                .wrap_err("failed to get channel balance")?;

            state
                .put_ibc_channel_balance(
                    self.source_channel(),
                    self.denom(),
                    channel_balance
                        .checked_add(self.amount())
                        .ok_or_eyre("overflow when adding to channel balance")?,
                )
                .wrap_err("failed to update channel balance")?;
        }

        state.send_packet_execute(packet).await;
        Ok(())
    }
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
    use astria_core::primitive::v1::RollupId;
    use ibc_types::core::client::Height;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        storage::Storage,
    };

    #[tokio::test]
    async fn sender_is_withdrawal_target_if_bridge_is_not_set_and_sender_is_not_bridge() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

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
            use_compat_address: false,
        };

        assert_eq!(
            *establish_withdrawal_target(&action, &state_delta, &from)
                .await
                .unwrap(),
            from
        );
    }

    #[tokio::test]
    async fn sender_is_withdrawal_target_if_bridge_is_unset_but_sender_is_bridge() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // sender is a bridge address, which is also the withdrawer, so it's ok
        let bridge_address = [1u8; 20];
        state_delta
            .put_bridge_account_rollup_id(
                &bridge_address,
                RollupId::from_unhashed_bytes("testrollupid"),
            )
            .unwrap();
        state_delta
            .put_bridge_account_withdrawer_address(&bridge_address, bridge_address)
            .unwrap();

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
            use_compat_address: false,
        };

        assert_eyre_error(
            &establish_withdrawal_target(&action, &state_delta, &bridge_address)
                .await
                .unwrap_err(),
            "sender cannot be a bridge address if bridge address is not set",
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
                use_compat_address: false,
            }
        }

        async fn run_test(action: action::Ics20Withdrawal) {
            let storage = Storage::new_temp().await;
            let mut state_delta = storage.new_delta_of_latest_snapshot();

            state_delta
                .put_base_prefix(ASTRIA_PREFIX.to_string())
                .unwrap();

            // withdraw is *not* the bridge address, Ics20Withdrawal must be sent by the withdrawer
            state_delta
                .put_bridge_account_rollup_id(
                    &bridge_address(),
                    RollupId::from_unhashed_bytes("testrollupid"),
                )
                .unwrap();
            state_delta
                .put_bridge_account_withdrawer_address(
                    &bridge_address(),
                    astria_address(&[2u8; 20]),
                )
                .unwrap();

            assert_eyre_error(
                &establish_withdrawal_target(&action, &state_delta, &bridge_address())
                    .await
                    .unwrap_err(),
                "sender does not match bridge withdrawer address; unauthorized",
            );
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();

        // sender the withdrawer address, so it's ok
        let bridge_address = [1u8; 20];
        let withdrawer_address = [2u8; 20];
        state_delta
            .put_bridge_account_rollup_id(
                &bridge_address,
                RollupId::from_unhashed_bytes("testrollupid"),
            )
            .unwrap();
        state_delta
            .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
            .unwrap();

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
            use_compat_address: false,
        };

        assert_eq!(
            *establish_withdrawal_target(&action, &state_delta, &withdrawer_address)
                .await
                .unwrap(),
            bridge_address,
        );
    }

    #[tokio::test]
    async fn bridge_is_rejected_as_withdrawal_target_because_it_has_no_withdrawer_address_set() {
        let storage = Storage::new_temp().await;
        let state_delta = storage.new_delta_of_latest_snapshot();

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
            use_compat_address: false,
        };

        assert_eyre_error(
            &establish_withdrawal_target(&action, &state_delta, &not_bridge_address)
                .await
                .unwrap_err(),
            "bridge address must have a withdrawer address set",
        );
    }
}
