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
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    bridge::state_ext::StateReadExt as _,
    ibc::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::action_handler::ActionHandler,
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

async fn ics20_withdrawal_check_stateful_bridge_account<S: StateReadExt + 'static>(
    action: &action::Ics20Withdrawal,
    state: &S,
    from: Address,
) -> Result<()> {
    // bridge address checks:
    // - if the sender of this transaction is not a bridge account, and the tx `bridge_address`
    //   field is None, don't need to do any bridge related checks as it's a normal user withdrawal.
    // - if the sender of this transaction is a bridge account, and the tx `bridge_address` field is
    //   None, check that the withdrawer address is the same as the transaction sender.
    // - if the tx `bridge_address` field is Some, check that the `bridge_address` is a valid
    //   bridge, and check that the withdrawer address is the same as the transaction sender.

    let is_sender_bridge = state
        .get_bridge_account_rollup_id(&from)
        .await
        .context("failed to get bridge account rollup id")?
        .is_some();

    if !is_sender_bridge && action.bridge_address.is_none() {
        return Ok(());
    }

    // if `action.bridge_address` is Some, but it's not a valid bridge account,
    // the `get_bridge_account_withdrawer_address` step will fail.
    let bridge_address = action.bridge_address.unwrap_or(from);

    let Some(withdrawer) = state
        .get_bridge_account_withdrawer_address(&bridge_address)
        .await
        .context("failed to get bridge withdrawer")?
    else {
        bail!("bridge address must have a withdrawer address set");
    };

    ensure!(
        withdrawer == from,
        "sender does not match bridge withdrawer address; unauthorized"
    );

    Ok(())
}

#[async_trait::async_trait]
impl ActionHandler for action::Ics20Withdrawal {
    #[instrument(skip(self))]
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.timeout_time() != 0, "timeout time must be non-zero",);

        // NOTE (from penumbra): we could validate the destination chain address as bech32 to
        // prevent mistyped addresses, but this would preclude sending to chains that don't
        // use bech32 addresses.
        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
    ) -> Result<()> {
        ics20_withdrawal_check_stateful_bridge_account(self, state, from).await?;

        let fee = state
            .get_ics20_withdrawal_base_fee()
            .await
            .context("failed to get ics20 withdrawal base fee")?;

        let packet: IBCPacket<Unchecked> = withdrawal_to_unchecked_ibc_packet(self);
        state
            .send_packet_check(packet)
            .await
            .context("packet failed send check")?;

        let transfer_asset_id = self.denom().id();

        let from_fee_balance = state
            .get_account_balance(from, *self.fee_asset_id())
            .await
            .context("failed getting `from` account balance for fee payment")?;

        // if fee asset is same as transfer asset, ensure accounts has enough funds
        // to cover both the fee and the amount transferred
        if self.fee_asset_id() == &transfer_asset_id {
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
                .get_account_balance(from, transfer_asset_id)
                .await
                .context("failed to get account balance in transfer check")?;
            ensure!(
                from_transfer_balance >= self.amount(),
                "insufficient funds for transfer"
            );
        }

        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: Address) -> Result<()> {
        let fee = state
            .get_ics20_withdrawal_base_fee()
            .await
            .context("failed to get ics20 withdrawal base fee")?;
        let checked_packet = withdrawal_to_unchecked_ibc_packet(self).assume_checked();

        state
            .decrease_balance(from, self.denom().id(), self.amount())
            .await
            .context("failed to decrease sender balance")?;

        state
            .decrease_balance(from, *self.fee_asset_id(), fee)
            .await
            .context("failed to subtract fee from sender balance")?;

        // if we're the source, move tokens to the escrow account,
        // otherwise the tokens are just burned
        if is_source(
            checked_packet.source_port(),
            checked_packet.source_channel(),
            self.denom(),
        ) {
            let channel_balance = state
                .get_ibc_channel_balance(self.source_channel(), self.denom().id())
                .await
                .context("failed to get channel balance")?;

            state
                .put_ibc_channel_balance(
                    self.source_channel(),
                    self.denom().id(),
                    channel_balance
                        .checked_add(self.amount())
                        .context("overflow when adding to channel balance")?,
                )
                .context("failed to update channel balance")?;
        }

        state.send_packet_execute(checked_packet).await;
        Ok(())
    }
}

fn is_source(source_port: &PortId, source_channel: &ChannelId, asset: &Denom) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");
    !asset.prefix_matches_exactly(&prefix)
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::RollupId;
    use cnidarium::StateDelta;
    use ibc_types::core::client::Height;

    use super::*;
    use crate::bridge::state_ext::StateWriteExt as _;

    #[tokio::test]
    async fn ics20_withdrawal_check_stateful_bridge_account_not_bridge() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let state = StateDelta::new(snapshot);

        let denom = "test".parse::<Denom>().unwrap();
        let from = crate::astria_address([1u8; 20]);
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: from,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
            memo: String::new(),
        };

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, from)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ics20_withdrawal_check_stateful_bridge_account_sender_is_bridge_bridge_address_none_ok()
     {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // sender is a bridge address, which is also the withdrawer, so it's ok
        let bridge_address = crate::astria_address([1u8; 20]);
        state.put_bridge_account_rollup_id(
            &bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(&bridge_address, &bridge_address);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: bridge_address,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
            memo: String::new(),
        };

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, bridge_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ics20_withdrawal_check_stateful_bridge_account_sender_is_bridge_bridge_address_none_invalid()
     {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // withdraw is *not* the bridge address, Ics20Withdrawal must be sent by the withdrawer
        let bridge_address = crate::astria_address([1u8; 20]);
        state.put_bridge_account_rollup_id(
            &bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(
            &bridge_address,
            &crate::astria_address([2u8; 20]),
        );

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: None,
            destination_chain_address: "test".to_string(),
            return_address: bridge_address,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
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
    async fn ics20_withdrawal_check_stateful_bridge_account_bridge_address_some_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // sender the withdrawer address, so it's ok
        let bridge_address = crate::astria_address([1u8; 20]);
        let withdrawer_address = crate::astria_address([2u8; 20]);
        state.put_bridge_account_rollup_id(
            &bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(&bridge_address, &withdrawer_address);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: Some(bridge_address),
            destination_chain_address: "test".to_string(),
            return_address: bridge_address,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
            memo: String::new(),
        };

        ics20_withdrawal_check_stateful_bridge_account(&action, &state, withdrawer_address)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn ics20_withdrawal_check_stateful_bridge_account_bridge_address_some_invalid_sender() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // sender is not the withdrawer address, so must fail
        let bridge_address = crate::astria_address([1u8; 20]);
        let withdrawer_address = crate::astria_address([2u8; 20]);
        state.put_bridge_account_rollup_id(
            &bridge_address,
            &RollupId::from_unhashed_bytes("testrollupid"),
        );
        state.put_bridge_account_withdrawer_address(&bridge_address, &withdrawer_address);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: Some(bridge_address),
            destination_chain_address: "test".to_string(),
            return_address: bridge_address,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
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
        let not_bridge_address = crate::astria_address([1u8; 20]);

        let denom = "test".parse::<Denom>().unwrap();
        let action = action::Ics20Withdrawal {
            amount: 1,
            denom: denom.clone(),
            bridge_address: Some(not_bridge_address),
            destination_chain_address: "test".to_string(),
            return_address: not_bridge_address,
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset_id: denom.id(),
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
