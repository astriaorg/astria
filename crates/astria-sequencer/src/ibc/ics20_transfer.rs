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
use std::borrow::Cow;

use anyhow::{
    bail,
    ensure,
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::{
        asset::{
            denom,
            Denom,
        },
        Address,
    },
    protocol::memos,
    sequencerblock::v1alpha1::block::Deposit,
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
    accounts::StateWriteExt as _,
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc,
    ibc::StateReadExt as _,
};

/// The maximum length of the encoded Ics20 `FungibleTokenPacketData` in bytes.
const MAX_PACKET_DATA_BYTE_LENGTH: usize = 2048;

/// The maximum length of the rollup address in bytes.
const MAX_ROLLUP_ADDRESS_BYTE_LENGTH: usize = 256;

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

    async fn recv_packet_check<S: StateRead>(_: S, msg: &MsgRecvPacket) -> Result<()> {
        // most checks performed in `execute`
        // perform stateless checks here
        if msg.packet.data.is_empty() {
            anyhow::bail!("packet data is empty");
        }

        if msg.packet.data.len() > MAX_PACKET_DATA_BYTE_LENGTH {
            anyhow::bail!("packet data is too long: exceeds MAX_PACKET_DATA_BYTE_LENGTH");
        }

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
    mut state: S,
    data: &[u8],
    source_port: &PortId,
    source_channel: &ChannelId,
) -> Result<()> {
    let packet_data: FungibleTokenPacketData =
        serde_json::from_slice(data).context("failed to decode fungible token packet data json")?;

    let denom = {
        let denom = packet_data
            .denom
            .parse::<Denom>()
            .context("failed parsing denom packet data")?;
        convert_denomination_if_ibc_prefixed(&mut state, denom)
            .await
            .context("failed to convert denomination if ibc/ prefixed")?
    };

    let is_source = !denom.starts_with_str(&format!("{source_port}/{source_channel}"));
    if is_source {
        // recipient of packet (us) was the source chain
        //
        // check if escrow account has enough balance to refund user
        let balance = state
            .get_ibc_channel_balance(source_channel, denom)
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
            Err(e) => {
                tracing::debug!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed to execute ics20 transfer"
                );
                TokenTransferAcknowledgement::Error(e.to_string())
            }
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

async fn convert_denomination_if_ibc_prefixed<S: ibc::StateReadExt>(
    state: &mut S,
    packet_denom: Denom,
) -> Result<denom::TracePrefixed> {
    // if the asset is prefixed with `ibc`, the rest of the denomination string is the asset ID,
    // so we need to look up the full trace from storage.
    // see https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-001-coin-source-tracing.md#decision
    let denom = match packet_denom {
        Denom::TracePrefixed(trace) => trace,
        Denom::IbcPrefixed(ibc) => state
            .map_ibc_to_trace_prefixed_asset(ibc)
            .await
            .context("failed to get denom trace from asset id")?
            .context("denom for given asset id not found in state")?,
    };
    Ok(denom)
}

fn prepend_denom_if_not_refund<'a>(
    packet_denom: &'a denom::TracePrefixed,
    dest_port: &PortId,
    dest_channel: &ChannelId,
    is_refund: bool,
) -> Cow<'a, denom::TracePrefixed> {
    if is_refund {
        Cow::Borrowed(packet_denom)
    } else {
        // FIXME: we should provide a method on `denom::TracePrefixed` to prepend segments to its
        // prefix
        let denom = format!("{dest_port}/{dest_channel}/{packet_denom}")
            .parse()
            .expect(
                "dest port and channel are valid prefix segments, so this concatenation must be a \
                 valid denom",
            );
        Cow::Owned(denom)
    }
}

// FIXME: temporarily allowed, but this must be fixed
#[allow(clippy::too_many_lines)]
async fn execute_ics20_transfer<S: ibc::StateWriteExt>(
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
        packet_data.sender.clone()
    } else {
        packet_data.receiver
    };

    let mut denom_trace = {
        let denom = packet_data
            .denom
            .parse::<Denom>()
            .context("failed parsing denom in packet data as Denom")?;
        // convert denomination if it's prefixed with `ibc/`
        // note: this denomination might have a prefix, but it wasn't prefixed by us right now.
        convert_denomination_if_ibc_prefixed(state, denom)
            .await
            .context("failed to convert denomination if ibc/ prefixed")?
    };

    // if the memo deserializes into an `Ics20WithdrawalFromRollupMemo`,
    // we can assume this is a refund from an attempted withdrawal from
    // a rollup directly to another IBC chain via the sequencer.
    //
    // in this case, we lock the tokens back in the bridge account and
    // emit a `Deposit` event to send the tokens back to the rollup.
    if is_refund
        && serde_json::from_str::<memos::v1alpha1::Ics20WithdrawalFromRollup>(&packet_data.memo)
            .is_ok()
    {
        let bridge_account = packet_data.sender.parse().context(
            "sender not an Astria Address: for refunds of ics20 withdrawals that came from a \
             rollup, the sender must be a valid Astria Address (usually the bridge account)",
        )?;
        execute_rollup_withdrawal_refund(
            state,
            bridge_account,
            &denom_trace,
            packet_amount,
            recipient,
        )
        .await
        .context("failed to execute rollup withdrawal refund")?;
        return Ok(());
    }

    // the IBC packet should have the address as a bech32 string
    let recipient = recipient.parse().context("invalid recipient address")?;

    let is_prefixed = denom_trace.starts_with_str(&format!("{source_port}/{source_channel}"));
    let is_source = if is_refund {
        // we are the source if the denom is not prefixed by source_port/source_channel
        !is_prefixed
    } else {
        // we are the source if the denom is prefixed by source_port/source_channel
        is_prefixed
    };

    // prefix the denomination with the destination port and channel if not a refund
    let trace_with_dest =
        prepend_denom_if_not_refund(&denom_trace, dest_port, dest_channel, is_refund);

    // check if this is a transfer to a bridge account and
    // execute relevant state changes if it is
    execute_ics20_transfer_bridge_lock(
        state,
        recipient,
        &trace_with_dest,
        packet_amount,
        packet_data.memo.clone(),
        is_refund,
    )
    .await
    .context("failed to execute ics20 transfer to bridge account")?;

    if is_source {
        // the asset being transferred in is an asset that originated from astria
        // subtract balance from escrow account and transfer to user

        // strip the prefix from the denom, as we're back on the source chain
        // note: if this is a refund, this is a no-op.
        if !is_refund {
            denom_trace.pop_trace_segment().context(
                "there must be a source segment because above it was checked if the denom trace \
                 contains a segment",
            )?;
        }
        let escrow_channel = if is_refund {
            source_channel
        } else {
            dest_channel
        };

        let escrow_balance = state
            .get_ibc_channel_balance(escrow_channel, &denom_trace)
            .await
            .context("failed to get IBC channel balance in execute_ics20_transfer")?;

        state
            .put_ibc_channel_balance(
                escrow_channel,
                &denom_trace,
                escrow_balance
                    .checked_sub(packet_amount)
                    .ok_or(anyhow::anyhow!(
                        "insufficient balance in escrow account to transfer tokens"
                    ))?,
            )
            .context("failed to update escrow account balance in execute_ics20_transfer")?;

        state
            .increase_balance(recipient, &denom_trace, packet_amount)
            .await
            .context("failed to update user account balance in execute_ics20_transfer")?;
    } else {
        // register denomination in global ID -> denom map if it's not already there
        if !state
            .has_ibc_asset(&*trace_with_dest)
            .await
            .context("failed to check if ibc asset exists in state")?
        {
            state
                .put_ibc_asset(&trace_with_dest)
                .context("failed to put IBC asset in storage")?;
        }

        state
            .increase_balance(recipient, &*trace_with_dest, packet_amount)
            .await
            .context("failed to update user account balance in execute_ics20_transfer")?;
    }

    Ok(())
}

/// execute a refund of tokens that were withdrawn from a rollup to another
/// IBC-enabled chain via the sequencer using an `Ics20Withdrawal`, but were not
/// transferred to the destination IBC chain successfully.
///
/// this functions sends the tokens back to the rollup via a `Deposit` event,
/// and locks the tokens back in the specified bridge account.
async fn execute_rollup_withdrawal_refund<S: ibc::StateWriteExt>(
    state: &mut S,
    bridge_address: Address,
    denom: &denom::TracePrefixed,
    amount: u128,
    destination_address: String,
) -> Result<()> {
    execute_deposit(state, bridge_address, denom, amount, destination_address).await?;

    state
        .increase_balance(bridge_address, denom, amount)
        .await
        .context(
            "failed to update bridge account account balance in execute_rollup_withdrawal_refund",
        )?;

    Ok(())
}

/// execute an ics20 transfer where the recipient is a bridge account.
///
/// if the recipient is not a bridge account, or the incoming packet is a refund,
/// this function is a no-op.
async fn execute_ics20_transfer_bridge_lock<S: ibc::StateWriteExt>(
    state: &mut S,
    recipient: Address,
    denom: &denom::TracePrefixed,
    amount: u128,
    memo: String,
    is_refund: bool,
) -> Result<()> {
    // check if the recipient is a bridge account; if so,
    // ensure that the packet memo field (`destination_address`) is set.
    let is_bridge_lock = state
        .get_bridge_account_rollup_id(recipient)
        .await
        .context("failed to get bridge account rollup ID from state")?
        .is_some();

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

    // assert memo is valid
    let deposit_memo: memos::v1alpha1::Ics20TransferDeposit =
        serde_json::from_str(&memo).context("failed to parse memo as Ics20TransferDepositMemo")?;

    ensure!(
        !deposit_memo.rollup_deposit_address.is_empty(),
        "rollup deposit address must be set to bridge funds from sequencer to rollup",
    );

    ensure!(
        deposit_memo.rollup_deposit_address.len() <= MAX_ROLLUP_ADDRESS_BYTE_LENGTH,
        "rollup address is too long: exceeds MAX_ROLLUP_ADDRESS_BYTE_LENGTH",
    );

    execute_deposit(
        state,
        recipient,
        denom,
        amount,
        deposit_memo.rollup_deposit_address,
    )
    .await
}

async fn execute_deposit<S: ibc::StateWriteExt>(
    state: &mut S,
    bridge_address: Address,
    denom: &denom::TracePrefixed,
    amount: u128,
    destination_address: String,
) -> Result<()> {
    // check if the recipient is a bridge account and
    // ensure that the asset ID being transferred
    // to it is allowed.
    let Some(rollup_id) = state
        .get_bridge_account_rollup_id(bridge_address)
        .await
        .context("failed to get bridge account rollup ID from state")?
    else {
        bail!("bridge account rollup ID not found in state; invalid bridge address?")
    };

    let allowed_asset = state
        .get_bridge_account_ibc_asset(bridge_address)
        .await
        .context("failed to get bridge account asset ID")?;
    ensure!(
        allowed_asset == denom.to_ibc_prefixed(),
        "asset ID is not authorized for transfer to bridge account",
    );

    let deposit = Deposit::new(
        bridge_address,
        rollup_id,
        amount,
        denom.into(),
        destination_address,
    );
    state
        .put_deposit_event(deposit)
        .await
        .context("failed to put deposit event into state")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use astria_core::primitive::v1::RollupId;
    use cnidarium::StateDelta;
    use denom::TracePrefixed;

    use super::*;
    use crate::{
        accounts::StateReadExt as _,
        ibc::StateWriteExt as _,
        test_utils::{
            astria_address,
            astria_address_from_hex_string,
        },
    };

    #[tokio::test]
    async fn prefix_denomination_not_refund() {
        let packet_denom = "asset".parse().unwrap();
        let dest_port = "transfer".to_string().parse().unwrap();
        let dest_channel = "channel-99".to_string().parse().unwrap();
        let is_refund = false;

        let denom =
            prepend_denom_if_not_refund(&packet_denom, &dest_port, &dest_channel, is_refund);
        let expected = "transfer/channel-99/asset"
            .parse::<TracePrefixed>()
            .unwrap();

        assert_eq!(denom.as_ref(), &expected);
    }

    #[tokio::test]
    async fn prefix_denomination_refund() {
        let packet_denom = "asset".parse::<TracePrefixed>().unwrap();
        let dest_port = "transfer".to_string().parse().unwrap();
        let dest_channel = "channel-99".to_string().parse().unwrap();
        let is_refund = true;

        let expected = packet_denom.clone();
        let denom =
            prepend_denom_if_not_refund(&packet_denom, &dest_port, &dest_channel, is_refund);
        assert_eq!(denom.as_ref(), &expected);
    }

    #[tokio::test]
    async fn convert_denomination_if_ibc_prefixed_with_prefix() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let denom_trace = "asset".parse().unwrap();
        state_tx.put_ibc_asset(&denom_trace).unwrap();

        let expected = denom_trace.clone();
        let packet_denom = denom_trace.to_ibc_prefixed().into();
        let denom = convert_denomination_if_ibc_prefixed(&mut state_tx, packet_denom)
            .await
            .unwrap();
        assert_eq!(denom, expected);
    }

    #[tokio::test]
    async fn convert_denomination_if_ibc_prefixed_without_prefix() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let packet_denom = "asset".parse::<Denom>().unwrap();
        let expected = packet_denom.clone().unwrap_trace_prefixed();
        let denom = convert_denomination_if_ibc_prefixed(&mut state_tx, packet_denom)
            .await
            .unwrap();
        assert_eq!(denom, expected);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_user_account() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let recipient = astria_address_from_hex_string("1c0c490f1b5528d8173c5de46d131160e4b2c0c3");
        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: recipient.to_string(),
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

        let denom = "dest_port/dest_channel/nootasset".parse::<Denom>().unwrap();
        let balance = state_tx.get_account_balance(recipient, denom).await.expect(
            "ics20 transfer to user account should succeed and balance should be minted to this \
             account",
        );
        assert_eq!(balance, 100);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_bridge_account_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom = "dest_port/dest_channel/nootasset".parse::<Denom>().unwrap();

        state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_ibc_asset(bridge_address, &denom)
            .unwrap();

        let memo = memos::v1alpha1::Ics20TransferDeposit {
            rollup_deposit_address: "rollupaddress".to_string(),
        };

        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: bridge_address.to_string(),
            memo: serde_json::to_string(&memo).unwrap(),
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

        let denom = "dest_port/dest_channel/nootasset".parse::<Denom>().unwrap();
        let balance = state_tx
            .get_account_balance(bridge_address, denom)
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

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom = "dest_port/dest_channel/nootasset".parse::<Denom>().unwrap();

        state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_ibc_asset(bridge_address, &denom)
            .unwrap();

        // use invalid memo, which should fail
        let packet = FungibleTokenPacketData {
            denom: "nootasset".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: bridge_address.to_string(),
            memo: "invalid".to_string(),
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

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom = "dest_port/dest_channel/nootasset".parse::<Denom>().unwrap();

        state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_ibc_asset(bridge_address, &denom)
            .unwrap();

        // use invalid asset, which should fail
        let packet = FungibleTokenPacketData {
            denom: "fake".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: bridge_address.to_string(),
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

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;
        let base_denom = "nootasset".parse::<Denom>().unwrap();
        state_tx
            .put_ibc_channel_balance(
                &"dest_channel".to_string().parse().unwrap(),
                &base_denom,
                amount,
            )
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: format!("source_port/source_channel/{base_denom}"),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
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

        let balance = state_tx
            .get_account_balance(recipient_address, &base_denom)
            .await
            .expect("ics20 transfer to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_tx
            .get_ibc_channel_balance(&"dest_channel".to_string().parse().unwrap(), &base_denom)
            .await
            .expect("ics20 transfer to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_to_user_account_is_source_refund() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;
        let base_denom = "nootasset".parse::<Denom>().unwrap();
        state_tx
            .put_ibc_channel_balance(
                &"source_channel".to_string().parse().unwrap(),
                &base_denom,
                amount,
            )
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: base_denom.to_string(),
            sender: recipient_address.to_string(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
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

        let balance = state_tx
            .get_account_balance(recipient_address, &base_denom)
            .await
            .expect("ics20 refund to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_tx
            .get_ibc_channel_balance(&"source_channel".to_string().parse().unwrap(), &base_denom)
            .await
            .expect("ics20 refund to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn execute_rollup_withdrawal_refund_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = astria_address(&[99u8; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");
        let denom = "dest_port/dest_channel/nootasset"
            .parse::<TracePrefixed>()
            .unwrap();

        state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_ibc_asset(bridge_address, &denom)
            .unwrap();

        let amount = 100;
        let destination_address = "destinationaddress".to_string();
        execute_rollup_withdrawal_refund(
            &mut state_tx,
            bridge_address,
            &denom,
            amount,
            destination_address,
        )
        .await
        .expect("valid rollup withdrawal refund");

        let balance = state_tx
            .get_account_balance(bridge_address, denom)
            .await
            .expect("rollup withdrawal refund should have updated funds in the bridge address");
        assert_eq!(balance, 100);

        let deposit = state_tx
            .get_block_deposits()
            .await
            .expect("a deposit should exist as a result of the rollup withdrawal refund");
        assert_eq!(deposit.len(), 1);
    }

    #[tokio::test]
    async fn execute_ics20_transfer_rollup_withdrawal_refund() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state_tx = StateDelta::new(snapshot.clone());

        let bridge_address = astria_address(&[99u8; 20]);
        let destination_chain_address = bridge_address.to_string();
        let denom = "nootasset".parse::<Denom>().unwrap();
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_tx.put_bridge_account_rollup_id(bridge_address, &rollup_id);
        state_tx
            .put_bridge_account_ibc_asset(bridge_address, &denom)
            .unwrap();

        let packet = FungibleTokenPacketData {
            denom: denom.to_string(),
            sender: bridge_address.to_string(),
            amount: "100".to_string(),
            receiver: "other-chain-address".to_string(),
            memo: serde_json::to_string(&memos::v1alpha1::Ics20WithdrawalFromRollup {
                memo: String::new(),
                rollup_block_number: 1,
                rollup_return_address: "rollup-defined".to_string(),
                rollup_transaction_hash: hex::encode([1u8; 32]),
            })
            .unwrap(),
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
        .expect("valid ics20 transfer refund; recipient, memo, and asset ID are valid");

        let balance = state_tx
            .get_account_balance(bridge_address, &denom)
            .await
            .expect(
                "ics20 transfer refunding to rollup should succeed and balance should be added to \
                 the bridge account",
            );
        assert_eq!(balance, 100);

        let deposits = state_tx
            .get_block_deposits()
            .await
            .expect("a deposit should exist as a result of the rollup withdrawal refund");
        assert_eq!(deposits.len(), 1);

        let deposit = deposits.get(&rollup_id).unwrap().first().unwrap();
        let expected_deposit = Deposit::new(
            bridge_address,
            rollup_id,
            100,
            denom,
            destination_chain_address,
        );
        assert_eq!(deposit, &expected_deposit);
    }
}
