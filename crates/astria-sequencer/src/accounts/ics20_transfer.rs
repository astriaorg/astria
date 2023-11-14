use anyhow::{
    Context,
    Result,
};
use ibc_types::{
    core::channel::{
        channel::Order as ChannelOrder,
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
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use proto::native::sequencer::v1alpha1::{
    Address,
    ADDRESS_LEN,
};

use super::state_ext::StateWriteExt;
use crate::accounts::state_ext::StateReadExt;

/// The ICS20 transfer handler.
/// See [here](https://github.com/cosmos/ibc/blob/main/spec/app/ics-020-fungible-token-transfer/README.md)
/// for the specification which this is based on.
#[derive(Clone)]
pub(crate) struct Ics20Transfer {}

#[async_trait::async_trait]
impl AppHandlerCheck for Ics20Transfer {
    async fn chan_open_init_check<S: StateRead>(_: S, msg: &MsgChannelOpenInit) -> Result<()> {
        if msg.ordering != ChannelOrder::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_proposal != "ics20-1".to_string().into() {
            anyhow::bail!("channel version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_try_check<S: StateRead>(_: S, msg: &MsgChannelOpenTry) -> Result<()> {
        if msg.ordering != ChannelOrder::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_supported_on_a != "ics20-1".to_string().into() {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_ack_check<S: StateRead>(_: S, msg: &MsgChannelOpenAck) -> Result<()> {
        if msg.version_on_b != "ics20-1".to_string().into() {
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
    }

    async fn acknowledge_packet_check<S: StateRead>(
        state: S,
        msg: &MsgAcknowledgement,
    ) -> Result<()> {
        // TODO: double check that this is correct format
        // see https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/modules/core/04-channel/types/acknowledgement.go
        // and https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/proto/ibc/core/channel/v1/channel.proto#L155
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
    }
}

fn refund_tokens_check<S: StateRead>(
    state: S,
    data: &[u8],
    source_port: &PortId,
    source_channel: &ChannelId,
) -> Result<()> {
    use prost::Message as _;

    let packet_data = FungibleTokenPacketData::decode(data)?;
    let denom = packet_data.denom;

    if is_source(source_port, source_channel, &denom, true) {
        // sender of packet (us) was the source chain
        //
        // check if escrow account has enough balance to refund user
        // TODO
        let balance = 0; //state.get_account_balance(source_channel, &denom).await?;

        let packet_amount: u128 = packet_data.amount.parse()?;
        if balance < packet_amount {
            anyhow::bail!("insufficient balance to refund tokens to sender");
        }
    }

    Ok(())
}

fn is_source(
    source_port: &PortId,
    source_channel: &ChannelId,
    denom: &str,
    is_refund: bool,
) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");
    if is_refund {
        !denom.starts_with(&prefix)
    } else {
        denom.starts_with(&prefix)
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
        _ = state
            .write_acknowledgement(&msg.packet, &ack_bytes)
            .await
            .map_err(|e| {
                tracing::error!("failed to write acknowledgement: {}", e);
            });
    }

    async fn timeout_packet_execute<S: StateWrite>(mut state: S, msg: &MsgTimeout) {
        // we put source and dest as chain_a (the source) as we're refunding tokens,
        // and the destination chain of the refund is the source.
        _ = execute_ics20_transfer(
            &mut state,
            &msg.packet.data,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            true,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "failed to refund tokens during timeout_packet_execute: {}",
                e
            );
        });
    }

    async fn acknowledge_packet_execute<S: StateWrite>(mut state: S, msg: &MsgAcknowledgement) {
        // TODO: double check that this is correct format
        // see https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/modules/core/04-channel/types/acknowledgement.go
        // and https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/proto/ibc/core/channel/v1/channel.proto#L155
        let ack: TokenTransferAcknowledgement = serde_json::from_slice(
            msg.acknowledgement.as_slice(),
        )
        .expect("valid acknowledgement, should have been checked in acknowledge_packet_check");
        if ack.is_successful() {
            return;
        }

        // we put source and dest as chain_a (the source) as we're refunding tokens,
        // and the destination chain of the refund is the source.
        _ = execute_ics20_transfer(
            &mut state,
            &msg.packet.data,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
            true,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "failed to refund tokens during acknowledge_packet_execute: {}",
                e
            );
        });
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
    use prost::Message as _;

    let packet_data = FungibleTokenPacketData::decode(data)?;
    let denom = packet_data.denom;
    let source_channel_address = address_from_channel_id(source_channel);
    let packet_amount: u128 = packet_data.amount.parse()?;
    let recipient = Address::try_from_slice(
        &hex::decode(packet_data.receiver).context("failed to decode receiver as hex string")?,
    )
    .context("invalid receiver address")?;

    if is_source(source_port, source_channel, &denom, is_refund) {
        // sender of packet (us) was the source chain
        // subtract balance from escrow account and transfer to user

        // TODO: get asset from dest_channel and denom
        let escrow_balance = state.get_account_balance(recipient).await?;
        let user_balance = state.get_account_balance(recipient).await?;
        state
            .put_account_balance(source_channel_address, escrow_balance - packet_amount)
            .context("failed to update escrow account balance")?;
        state
            .put_account_balance(recipient, user_balance + packet_amount)
            .context("failed to update user account balance")?;
    } else {
        let prefixed_denomination = if is_refund {
            // we're refunding a token we issued and tried to bridge, but failed
            denom
        } else {
            // we're receiving a token from another chain
            // create a token with additional prefix and mint it to the recipient
            format!("{dest_port}/{dest_channel}/{denom}")
        };

        // TODO: register denomination in global ID -> denom map
        // if it's not already there

        // TODO: use prefixed_denomination
        let user_balance = state.get_account_balance(recipient).await?;
        state
            .put_account_balance(recipient, user_balance + packet_amount)
            .context("failed to update user account balance")?;
    }

    Ok(())
}

fn address_from_channel_id(channel_id: &ChannelId) -> Address {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(channel_id.as_bytes());
    let bytes: [u8; 32] = hasher.finalize().into();
    Address::try_from_slice(&bytes[..ADDRESS_LEN])
        .expect("can convert 32 byte hash to 20 byte array")
}
