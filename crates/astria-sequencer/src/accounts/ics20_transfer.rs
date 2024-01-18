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
    Context as _,
    Result,
};
use astria_core::sequencer::v1alpha1::{
    asset::IbcAsset,
    Address,
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
use penumbra_proto::penumbra::core::component::ibc::v1alpha1::FungibleTokenPacketData;

use super::state_ext::{
    StateReadExt as _,
    StateWriteExt,
};
use crate::asset::state_ext::{
    StateReadExt as _,
    StateWriteExt as _,
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
    use prost::Message as _;

    let packet_data = FungibleTokenPacketData::decode(data)
        .context("failed to decode fungible token packet data json")?;
    let mut asset = packet_data
        .denom
        .parse::<IbcAsset>()
        .context("failed parsing `denom` field packet data as IbcAsset")?;

    // if the asset is prefixed with `ibc`, the rest of the denomination string is the asset ID,
    // so we need to look up the full trace from storage.
    // see https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-001-coin-source-tracing.md#decision
    if asset.prefix_is("ibc") {
        asset = state
            .get_ibc_asset(asset.id())
            .await
            .context("failed to get denom trace from asset id")?;
    }

    if is_source(source_port, source_channel, &asset, true) {
        // sender of packet (us) was the source chain
        //
        // check if escrow account has enough balance to refund user
        let balance = state
            .get_ibc_channel_balance(source_channel, asset.id())
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

fn is_source(
    source_port: &PortId,
    source_channel: &ChannelId,
    asset: &IbcAsset,
    is_refund: bool,
) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");
    if is_refund {
        !asset.prefix_is(&prefix)
    } else {
        asset.prefix_is(&prefix)
    }
}

#[async_trait::async_trait]
impl AppHandlerExecute for Ics20Transfer {
    async fn chan_open_init_execute<S: StateWrite>(_: S, _: &MsgChannelOpenInit) {}

    async fn chan_open_try_execute<S: StateWrite>(_: S, _: &MsgChannelOpenTry) {}

    async fn chan_open_ack_execute<S: StateWrite>(_: S, _: &MsgChannelOpenAck) {}

    async fn chan_open_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelOpenConfirm) {}

    async fn chan_close_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelCloseConfirm) {}

    async fn chan_close_init_execute<S: StateWrite>(_: S, _: &MsgChannelCloseInit) {}

    async fn recv_packet_execute<S: StateWrite>(mut state: S, msg: &MsgRecvPacket) {
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

        if let Err(e) = state.write_acknowledgement(&msg.packet, &ack_bytes).await {
            let error: &dyn std::error::Error = e.as_ref();
            tracing::error!(error, "failed to write acknowledgement");
        }
    }

    async fn timeout_packet_execute<S: StateWrite>(mut state: S, msg: &MsgTimeout) {
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
                "failed to refund tokens during timeout_packet_execute",
            );
        };
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
    let recipient = Address::try_from_slice(
        &hex::decode(packet_data.receiver).context("failed to decode receiver as hex string")?,
    )
    .context("invalid receiver address")?;
    let mut asset = packet_data
        .denom
        .parse::<IbcAsset>()
        .context("failed parsing `denom` field packet data as IbcAsset")?;

    // if the asset is prefixed with `ibc`, the rest of the denomination string is the asset ID,
    // so we need to look up the full trace from storage.
    // see https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-001-coin-source-tracing.md#decision
    if asset.prefix_is("ibc") {
        asset = state
            .get_ibc_asset(asset.id())
            .await
            .context("failed to get denom trace from asset id")?;
    }

    if is_source(source_port, source_channel, &asset, is_refund) {
        // sender of packet (us) was the source chain
        // subtract balance from escrow account and transfer to user

        let escrow_balance = state
            .get_ibc_channel_balance(source_channel, asset.id())
            .await
            .context("failed to get IBC channel balance in execute_ics20_transfer")?;

        let user_balance = state.get_account_balance(recipient, asset.id()).await?;
        state
            .put_ibc_channel_balance(
                source_channel,
                asset.id(),
                escrow_balance
                    .checked_sub(packet_amount)
                    .ok_or(anyhow::anyhow!(
                        "insufficient balance in escrow account to transfer tokens"
                    ))?,
            )
            .context("failed to update escrow account balance in execute_ics20_transfer")?;
        state
            .put_account_balance(
                recipient,
                asset.id(),
                user_balance
                    .checked_add(packet_amount)
                    .ok_or(anyhow::anyhow!("overflow when adding to user balance"))?,
            )
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

        let asset: IbcAsset = prefixed_denomination
            .parse()
            .context("failed to parse prefixed denomination as IbcAsset")?;

        // register denomination in global ID -> denom map if it's not already there
        if !state
            .has_ibc_asset(asset.id())
            .await
            .context("failed to check if ibc asset exists in state")?
        {
            state
                .put_ibc_asset(asset.id(), &asset)
                .context("failed to put IBC asset in storage")?;
        }

        let user_balance = state
            .get_account_balance(recipient, asset.id())
            .await
            .context("failed to get user account balance in execute_ics20_transfer")?;
        state
            .put_account_balance(
                recipient,
                asset.id(),
                user_balance
                    .checked_add(packet_amount)
                    .ok_or(anyhow::anyhow!("overflow when adding to user balance"))?,
            )
            .context("failed to update user account balance in execute_ics20_transfer")?;
    }

    Ok(())
}
