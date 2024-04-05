//! This module implements the ICS20 transfer handler, which handles
//! incoming packets.
//!
//! It contains an [`Ics20Transfer`] struct which implements the Penumbra
//! [`AppHandler`] trait, which is passed through the Penumbra IBC implementation
//! during transaction checks and execution. The IBC implementation calls into
//! the ICS20 transfer handler during the IBC transaction lifecycle.
//!
//! [`AppHandler`] consists of two traits: [`AppHandlerCheck`] and [`AppHandlerExecute`].
//! [`AppHandlerCheck`] is used for stateless and stateful checks, while
//! [`AppHandlerExecute`] is used for execution.

use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_core::sequencer::{
    v1::{
        asset::Denom,
        Address,
    },
    v2::block::Deposit,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use ibc_types::{
    core::channel::{
        channel,
        msgs::{
            MsgAcknowledgement,
            MsgChannelCloseConfirm,
            MsgChannelCloseInit,
            MsgChannelOpenAck,
            MsgChannelOpenConfirm,
            MsgChannelOpenInit,
            MsgChannelOpenTry,
            MsgRecvPacket,
            MsgTimeout,
        },
        ChannelId,
        PortId,
    },
    transfer::acknowledgement::TokenTransferAcknowledgement,
};
use penumbra_ibc::component::app_handler::{
    AppHandler,
    AppHandlerCheck,
    AppHandlerExecute,
};
use penumbra_proto::penumbra::core::component::ibc::v1::FungibleTokenPacketData;

use crate::{
    accounts::state_ext::StateWriteExt as _,
    asset::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::state_ext::{
        StateReadExt as _,
        StateWriteExt,
    },
};

/// The ICS20 transfer handler.
///
/// See [here](https://github.com/cosmos/ibc/blob/main/spec/app/ics-020-fungible-token-transfer/README.md)
/// for the specification which this is based on.
#[derive(Clone)]
pub(crate) struct Ics20Transfer;

#[async_trait::async_trait]
impl AppHandlerCheck for Ics20Transfer {
    async fn chan_open_init_check<S: StateRead>(_: S, msg: &MsgChannelOpenInit) -> Result<()> {
        if msg.ordering != channel::Order::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_proposal.as_str() != "ics20-1" {
            anyhow::bail!("channel version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_try_check<S: StateRead>(_: S, msg: &MsgChannelOpenTry) -> Result<()> {
        if msg.ordering != channel::Order::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_supported_on_a.as_str() != "ics20-1" {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_ack_check<S: StateRead>(_: S, msg: &MsgChannelOpenAck) -> Result<()> {
        if msg.version_on_b.as_str() != "ics20-1" {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_confirm_check<S: StateRead>(_: S, _: &MsgChannelOpenConfirm) -> Result<()> {
        // accept channel confirmations, port has already been validated, version has already been
        // validated
        Ok(())
    }

    async fn chan_close_init_check<S: StateRead>(_: S, _: &MsgChannelCloseInit) -> Result<()> {
        anyhow::bail!("ics20 always aborts on chan_close_init");
    }

    async fn chan_close_confirm_check<S: StateRead>(
        _: S,
        _: &MsgChannelCloseConfirm,
    ) -> Result<()> {
        // no action needed
        Ok(())
    }

    async fn recv_packet_check<S: StateRead>(_: S, _: &MsgRecvPacket) -> Result<()> {
        // checks performed in `execute`
        Ok(())
    }

    async fn timeout_packet_check<S: StateRead>(state: S, msg: &MsgTimeout) -> Result<()> {
        refund_tokens_check(
            state,
            msg.packet.data.as_slice(),
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
        )
        .await
    }

    async fn acknowledge_packet_check<S: StateRead>(
        state: S,
        msg: &MsgAcknowledgement,
    ) -> Result<()> {
        // see https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/modules/core/04-channel/types/acknowledgement.go
        // and https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/proto/ibc/core/channel/v1/channel.proto#L155
        // for formatting
        let ack: TokenTransferAcknowledgement =
            serde_json::from_slice(msg.acknowledgement.as_slice())?;
        if ack.is_successful() {
            return Ok(());
        }

        refund_tokens_check(
            state,
            msg.packet.data.as_slice(),
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
        )
        .await
    }
}

async fn refund_tokens_check<S: StateRead>(
    state: S,
    data: &[u8],
    source_port: &PortId,
    source_channel: &ChannelId,
) -> Result<()> {
    let packet_data: FungibleTokenPacketData =
        serde_json::from_slice(data).context("failed to decode fungible token packet data json")?;
    let mut denom: Denom = packet_data.denom.clone().into();

    // if the asset is prefixed with `ibc`, the rest of the denomination string is the asset ID,
    // so we need to look up the full trace from storage.
    // see https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-001-coin-source-tracing.md#decision
    if denom.prefix_is("ibc") {
        denom = state
            .get_ibc_asset(denom.id())
            .await
            .context("failed to get denom trace from asset id")?;
    }

    let is_source = !is_prefixed(source_port, source_channel, &denom);
    if is_source {
        // recipient of packet (us) was the source chain
        //
        // check if escrow account has enough balance to refund user
        let balance = state
            .get_ibc_channel_balance(source_channel, denom.id())
            .await
            .context("failed to get channel balance in refund_tokens_check")?;

        let packet_amount: u128 = packet_data
            .amount
            .parse()
            .context("failed to parse packet amount as u128")?;
        if balance < packet_amount {
            anyhow::bail!("insufficient balance to refund tokens to sender");
        }
    }

    Ok(())
}

fn is_prefixed(source_port: &PortId, source_channel: &ChannelId, asset: &Denom) -> bool {
    let prefix = format!("{source_port}/{source_channel}");
    asset.prefix_is(&prefix)
}

#[async_trait::async_trait]
impl AppHandlerExecute for Ics20Transfer {
    async fn chan_open_init_execute<S: StateWrite>(_: S, _: &MsgChannelOpenInit) {}

    async fn chan_open_try_execute<S: StateWrite>(_: S, _: &MsgChannelOpenTry) {}

    async fn chan_open_ack_execute<S: StateWrite>(_: S, _: &MsgChannelOpenAck) {}

    async fn chan_open_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelOpenConfirm) {}

    async fn chan_close_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelCloseConfirm) {}

    async fn chan_close_init_execute<S: StateWrite>(_: S, _: &MsgChannelCloseInit) {}

    async fn recv_packet_execute<S: StateWrite>(
        mut state: S,
        msg: &MsgRecvPacket,
    ) -> anyhow::Result<()> {
        use penumbra_ibc::component::packet::WriteAcknowledgement as _;

        let ack = match execute_ics20_transfer(
            &mut state,
            &msg.packet.data,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            &msg.packet.port_on_b,
            &msg.packet.chan_on_b,
            false,
        )
        .await
        {
            Ok(()) => TokenTransferAcknowledgement::success(),
            Err(e) => TokenTransferAcknowledgement::Error(e.to_string()),
        };

        let ack_bytes: Vec<u8> = ack.into();

        state
            .write_acknowledgement(&msg.packet, &ack_bytes)
            .await
            .context("failed to write acknowledgement")
    }

    async fn timeout_packet_execute<S: StateWrite>(
        mut state: S,
        msg: &MsgTimeout,
    ) -> anyhow::Result<()> {
        // we put source and dest as chain_a (the source) as we're refunding tokens,
        // and the destination chain of the refund is the source.
        execute_ics20_transfer(
            &mut state,
            &msg.packet.data,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            true,
        )
        .await
        .context("failed to refund tokens during timeout_packet_execute")
    }

    async fn acknowledge_packet_execute<S: StateWrite>(mut state: S, msg: &MsgAcknowledgement) {
        let ack: TokenTransferAcknowledgement = serde_json::from_slice(
            msg.acknowledgement.as_slice(),
        )
        .expect("valid acknowledgement, should have been checked in acknowledge_packet_check");
        if ack.is_successful() {
            return;
        }

        // we put source and dest as chain_a (the source) as we're refunding tokens,
        // and the destination chain of the refund is the source.
        if let Err(e) = execute_ics20_transfer(
            &mut state,
            &msg.packet.data,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            true,
        )
        .await
        {
            let error: &dyn std::error::Error = e.as_ref();
            tracing::error!(
                error,
                "failed to refund tokens during acknowledge_packet_execute",
            );
        }
    }
}

#[async_trait::async_trait]
impl AppHandler for Ics20Transfer {}

async fn execute_ics20_transfer_bridge_lock<S: StateWriteExt>(
    state: &mut S,
    recipient: &Address,
    denom: &Denom,
    amount: u128,
    destination_address: String,
    is_refund: bool,
) -> Result<()> {
    // check if the recipient is a bridge account; if so,
    // ensure that the packet memo field (`destination_address`) is set.
    //
    // also, ensure that the asset ID being transferred
    // to it is allowed.
    let maybe_recipient_rollup_id = state
        .get_bridge_account_rollup_id(recipient)
        .await
        .context("failed to get bridge account rollup ID from state")?;
    let is_bridge_lock = maybe_recipient_rollup_id.is_some();

    // if account being transferred to is not a bridge account, or
    // the incoming packet is a refund, return
    //
    // note on refunds: bridge accounts *are* allowed to do ICS20 withdrawals,
    // so this could be a refund to a bridge account if that withdrawal times out.
    //
    // so, if this is a refund transaction, we don't need to emit a `Deposit`,
    // as the tokens are being refunded to the bridge's account.
    //
    // then, we don't need to check the memo field (as no `Deposit` is created),
    // or check the asset IDs (as the asset IDs that can be sent out are the same
    // as those that can be received).
    if !is_bridge_lock || is_refund {
        return Ok(());
    }

    ensure!(
        !destination_address.is_empty(),
        "packet memo field must be set for bridge account recipient",
    );

    let allowed_asset_ids = state
        .get_bridge_account_asset_ids(recipient)
        .await
        .context("failed to get bridge account asset IDs")?;
    ensure!(
        allowed_asset_ids.contains(&denom.id()),
        "asset ID is not authorized for transfer to bridge account",
    );

    let deposit = Deposit::new(
        *recipient,
        maybe_recipient_rollup_id
            .expect("recipient has a rollup ID; this was checked via `is_bridge_lock`"),
        amount,
        denom.id(),
        destination_address,
    );
    state
        .put_deposit_event(deposit)
        .await
        .context("failed to put deposit event into state")?;

    Ok(())
}

async fn execute_ics20_transfer<S: StateWriteExt>(
    state: &mut S,
    data: &[u8],
    source_port: &PortId,
    source_channel: &ChannelId,
    dest_port: &PortId,
    dest_channel: &ChannelId,
    is_refund: bool,
) -> Result<()> {
    let packet_data: FungibleTokenPacketData =
        serde_json::from_slice(data).context("failed to decode FungibleTokenPacketData")?;
    let packet_amount: u128 = packet_data
        .amount
        .parse()
        .context("failed to parse packet data amount to u128")?;
    let recipient = if is_refund {
        packet_data.sender
    } else {
        packet_data.receiver
    };

    let recipient = Address::try_from_slice(
        &hex::decode(recipient).context("failed to decode recipient as hex string")?,
    )
    .context("invalid recipient address")?;
    let mut denom: Denom = packet_data.denom.clone().into();

    // if the asset is prefixed with `ibc`, the rest of the denomination string is the asset ID,
    // so we need to look up the full trace from storage.
    // see https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-001-coin-source-tracing.md#decision
    if denom.prefix().starts_with("ibc") {
        denom = state
            .get_ibc_asset(denom.id())
            .await
            .context("failed to get denom trace from asset id")?;
    }

    // check if this is a transfer to a bridge account and
    // execute relevant state changes if it is
    execute_ics20_transfer_bridge_lock(
        state,
        &recipient,
        &denom,
        packet_amount,
        packet_data.memo.clone(),
        is_refund,
    )
    .await
    .context("failed to execute ics20 transfer to bridge account")?;

    let is_prefixed = is_prefixed(source_port, source_channel, &denom);
    let is_source = if is_refund {
        // we are the source if the denom is not prefixed by source_port/source_channel
        !is_prefixed
    } else {
        // we are the source if the denom is prefixed by source_port/source_channel
        is_prefixed
    };

    if is_source {
        // the asset being transferred in is an asset that originated from astria
        // subtract balance from escrow account and transfer to user

        // strip the prefix from the denom, as we're back on the source chain
        // note: if this is a refund, this is a no-op.
        let denom = denom.to_base_denom();

        let escrow_channel = if is_refund {
            source_channel
        } else {
            dest_channel
        };

        let escrow_balance = state
            .get_ibc_channel_balance(escrow_channel, denom.id())
            .await
            .context("failed to get IBC channel balance in execute_ics20_transfer")?;

        state
            .put_ibc_channel_balance(
                escrow_channel,
                denom.id(),
                escrow_balance
                    .checked_sub(packet_amount)
                    .ok_or(anyhow::anyhow!(
                        "insufficient balance in escrow account to transfer tokens"
                    ))?,
            )
            .context("failed to update escrow account balance in execute_ics20_transfer")?;

        state
            .increase_balance(recipient, denom.id(), packet_amount)
            .await
            .context("failed to update user account balance in execute_ics20_transfer")?;
    } else {
        let prefixed_denomination = if is_refund {
            // we're refunding a token we issued and tried to bridge, but failed
            packet_data.denom
        } else {
            // we're receiving a token from another chain
            // create a token with additional prefix and mint it to the recipient
            format!("{dest_port}/{dest_channel}/{}", packet_data.denom)
        };

        let denom: Denom = prefixed_denomination.into();

        // register denomination in global ID -> denom map if it's not already there
        if !state
            .has_ibc_asset(denom.id())
            .await
            .context("failed to check if ibc asset exists in state")?
        {
            state
                .put_ibc_asset(denom.id(), &denom)
                .context("failed to put IBC asset in storage")?;
        }

        state
            .increase_balance(recipient, denom.id(), packet_amount)
            .await
            .context("failed to update user account balance in execute_ics20_transfer")?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use astria_core::sequencer::v1::RollupId;
    use cnidarium::StateDelta;

    use super::*;
    use crate::accounts::state_ext::StateReadExt as _;

    #[test]
    fn is_prefixed_test() {
        let source_port = "source_port".to_string().parse().unwrap();
        let source_channel = "source_channel".to_string().parse().unwrap();
        let asset = Denom::from("source_port/source_channel/asset".to_string());
        // in the case of a transfer in that is not a refund,
        // we are the source if the packets are prefixed by the sending chain
        assert!(is_prefixed(&source_port, &source_channel, &asset));
        // in the case of a refund, we are the source if the packets are not
        // prefixed by the sending chain
        let asset = Denom::from("other_port/source_channel/asset".to_string());
        assert!(!is_prefixed(&source_port, &source_channel, &asset));
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_user_account() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: "1c0c490f1b5528d8173c5de46d131160e4b2c0c3".to_string(),
            memo: String::new(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"dest_port".to_string().parse().unwrap(),
            &"dest_channel".to_string().parse().unwrap(),
            false,
        )
        .await
        .expect("valid ics20 transfer to user account; recipient, memo, and asset ID are valid");

        let recipient = Address::try_from_slice(
            &hex::decode("1c0c490f1b5528d8173c5de46d131160e4b2c0c3").unwrap(),
        )
        .unwrap();
        let denom: Denom = format!("dest_port/dest_channel/{}", "nootasset").into();
        let balance = state_tx
            .get_account_balance(recipient, denom.id())
            .await
            .expect(
                "ics20 transfer to user account should succeed and balance should be minted to \
                 this account",
            );
        assert_eq!(balance, 100);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_bridge_account_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom: Denom = "nootasset".to_string().into();

        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[denom.id()])
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: hex::encode(bridge_address),
            memo: "destinationaddress".to_string(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"dest_port".to_string().parse().unwrap(),
            &"dest_channel".to_string().parse().unwrap(),
            false,
        )
        .await
        .expect("valid ics20 transfer to bridge account; recipient, memo, and asset ID are valid");

        let denom: Denom = format!("dest_port/dest_channel/{}", "nootasset").into();
        let balance = state_tx
            .get_account_balance(bridge_address, denom.id())
            .await
            .expect(
                "ics20 transfer from sender to bridge account should have updated funds in the \
                 bridge address",
            );
        assert_eq!(balance, 100);

        let deposit = state_tx
            .get_block_deposits()
            .await
            .expect("a deposit should exist as a result of the transfer to a bridge account");
        assert_eq!(deposit.len(), 1);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_bridge_account_invalid_memo() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom: Denom = "nootasset".to_string().into();

        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[denom.id()])
            .unwrap();

        // use empty memo, which should fail
        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: hex::encode(bridge_address),
            memo: String::new(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"dest_port".to_string().parse().unwrap(),
            &"dest_channel".to_string().parse().unwrap(),
            false,
        )
        .await
        .expect_err("empty packet memo field during transfer to bridge account should fail");
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_bridge_account_invalid_asset() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = Address::from([99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom: Denom = "nootasset".to_string().into();

        state_tx.put_bridge_account_rollup_id(&bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_asset_ids(&bridge_address, &[denom.id()])
            .unwrap();

        // use invalid asset, which should fail
        let packet = FungibleTokenPacketData {
            denom: "fake".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: hex::encode(bridge_address),
            memo: "destinationaddress".to_string(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"dest_port".to_string().parse().unwrap(),
            &"dest_channel".to_string().parse().unwrap(),
            false,
        )
        .await
        .expect_err("invalid asset during transfer to bridge account should fail");
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_user_account_is_source_not_refund() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let address_string = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
        let amount = 100;
        let base_denom: Denom = "nootasset".to_string().into();
        state_tx
            .put_ibc_channel_balance(
                &"dest_channel".to_string().parse().unwrap(),
                base_denom.id(),
                amount,
            )
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: format!("source_port/source_channel/{base_denom}"),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: address_string.to_string(),
            memo: String::new(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"dest_port".to_string().parse().unwrap(),
            &"dest_channel".to_string().parse().unwrap(),
            false,
        )
        .await
        .expect("valid ics20 transfer to user account; recipient, memo, and asset ID are valid");

        let recipient = Address::try_from_slice(&hex::decode(address_string).unwrap()).unwrap();
        let balance = state_tx
            .get_account_balance(recipient, base_denom.id())
            .await
            .expect("ics20 transfer to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_tx
            .get_ibc_channel_balance(
                &"dest_channel".to_string().parse().unwrap(),
                base_denom.id(),
            )
            .await
            .expect("ics20 transfer to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_user_account_is_source_refund() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let address_string = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
        let amount = 100;
        let base_denom: Denom = "nootasset".to_string().into();
        state_tx
            .put_ibc_channel_balance(
                &"source_channel".to_string().parse().unwrap(),
                base_denom.id(),
                amount,
            )
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: base_denom.to_string(),
            sender: address_string.to_string(),
            amount: amount.to_string(),
            receiver: address_string.to_string(),
            memo: String::new(),
        };
        let packet_bytes = serde_json::to_vec(&packet).unwrap();

        execute_ics20_transfer(
            &mut state_tx,
            &packet_bytes,
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            &"source_port".to_string().parse().unwrap(),
            &"source_channel".to_string().parse().unwrap(),
            true,
        )
        .await
        .expect("valid ics20 refund to user account; recipient, memo, and asset ID are valid");

        let recipient = Address::try_from_slice(&hex::decode(address_string).unwrap()).unwrap();
        let balance = state_tx
            .get_account_balance(recipient, base_denom.id())
            .await
            .expect("ics20 refund to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_tx
            .get_ibc_channel_balance(
                &"source_channel".to_string().parse().unwrap(),
                base_denom.id(),
            )
            .await
            .expect("ics20 refund to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }
}
