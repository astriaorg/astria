use anyhow::{
    anyhow,
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
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::ActionHandler,
    bridge::StateReadExt as _,
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
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

async fn ics20_withdrawal_check_stateful_bridge_account<S: StateRead>(
    action: &action::Ics20Withdrawal,
    state: &S,
    from: [u8; 20],
) -> Result<()> {
    // bridge address checks:
    // - if the sender of this transaction is not a bridge account, and the tx `bridge_address`
    //   field is None, don't need to do any bridge related checks as it's a normal user withdrawal.
    // - if the sender of this transaction is a bridge account, and the tx `bridge_address` field is
    //   None, check that the withdrawer address is the same as the transaction sender.
    // - if the tx `bridge_address` field is Some, check that the `bridge_address` is a valid
    //   bridge, and check that the withdrawer address is the same as the transaction sender.

    let is_sender_bridge = state
        .get_bridge_account_rollup_id(from)
        .await
        .context("failed to get bridge account rollup id")?
        .is_some();

    if !is_sender_bridge && action.bridge_address.is_none() {
        return Ok(());
    }

    // if `action.bridge_address` is Some, but it's not a valid bridge account,
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

    Ok(())
}

#[async_trait::async_trait]
impl ActionHandler for action::Ics20Withdrawal {
    type CheckStatelessContext = ();

    async fn check_stateless(&self, _context: Self::CheckStatelessContext) -> Result<()> {
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

        ics20_withdrawal_check_stateful_bridge_account(self, &state, from).await?;

        let fee = state
            .get_ics20_withdrawal_base_fee()
            .await
            .context("failed to get ics20 withdrawal base fee")?;

        let packet = {
            let packet = withdrawal_to_unchecked_ibc_packet(self);
            state
                .send_packet_check(packet)
                .await
                .context("packet failed send check")?
        };

        let transfer_asset = self.denom();

        let from_fee_balance = state
            .get_account_balance(from, self.fee_asset())
            .await
            .context("failed getting `from` account balance for fee payment")?;

        // if fee asset is same as transfer asset, ensure accounts has enough funds
        // to cover both the fee and the amount transferred
        if self.fee_asset().to_ibc_prefixed() == transfer_asset.to_ibc_prefixed() {
            let payment_amount = self
                .amount()
                .checked_add(fee)
                .ok_or(anyhow!("transfer amount plus fee overflowed"))?;

            ensure!(
                from_fee_balance >= payment_amount,
                "insufficient funds for transfer and fee payment"
            );
        } else {
            // otherwise, check the fee asset account has enough to cover the fees,
            // and the transfer asset account has enough to cover the transfer
            ensure!(
                from_fee_balance >= fee,
                "insufficient funds for fee payment"
            );

            let from_transfer_balance = state
                .get_account_balance(from, transfer_asset)
                .await
                .context("failed to get account balance in transfer check")?;
            ensure!(
                from_transfer_balance >= self.amount(),
                "insufficient funds for transfer"
            );
        }

        state
            .decrease_balance(from, self.denom(), self.amount())
            .await
            .context("failed to decrease sender balance")?;

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
            astria_address,
            ASTRIA_PREFIX,
        },
    };

    #[tokio::test]
    async fn check_stateful_bridge_account_not_bridge() {
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

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, from)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn check_stateful_bridge_account_sender_is_bridge_bridge_address_none_ok() {
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

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, bridge_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn check_stateful_bridge_account_sender_is_bridge_bridge_address_none_invalid() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        // withdraw is *not* the bridge address, Ics20Withdrawal must be sent by the withdrawer
        let bridge_address = [1u8; 20];
        state.put_bridge_account_rollup_id(
            bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(bridge_address, astria_address(&[2u8; 20]));

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

        let err = ics20_withdrawal_check_stateful_bridge_account(&action, &state, bridge_address)
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("sender does not match bridge withdrawer address; unauthorized")
        );
    }

    #[tokio::test]
    async fn check_stateful_bridge_account_bridge_address_some_ok() {
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

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, withdrawer_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn check_stateful_bridge_account_bridge_address_some_invalid_sender() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX).unwrap();

        // sender is not the withdrawer address, so must fail
        let bridge_address = [1u8; 20];
        let withdrawer_address = astria_address(&[2u8; 20]);
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

        let err = ics20_withdrawal_check_stateful_bridge_account(&action, &state, bridge_address)
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("sender does not match bridge withdrawer address; unauthorized")
        );
    }

    #[tokio::test]
    async fn ics20_withdrawal_check_stateful_bridge_account_bridge_address_some_invalid_bridge_account()
     {
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

        let err =
            ics20_withdrawal_check_stateful_bridge_account(&action, &state, not_bridge_address)
                .await
                .unwrap_err();
        assert!(
            err.to_string()
                .contains("bridge address must have a withdrawer address set")
        );
    }
}
