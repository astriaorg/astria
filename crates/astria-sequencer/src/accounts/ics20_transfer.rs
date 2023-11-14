use anyhow::Result;
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
use penumbra_ibc::component::{
    app_handler::{
        AppHandler,
        AppHandlerCheck,
        AppHandlerExecute,
    },
    packet::{
        IBCPacket,
        SendPacketRead as _,
        SendPacketWrite as _,
        Unchecked,
        WriteAcknowledgement as _,
    },
    state_key,
};
use penumbra_proto::{
    penumbra::core::component::ibc::v1alpha1::FungibleTokenPacketData,
    StateReadProto,
    StateWriteProto,
};
use penumbra_storage::{
    StateRead,
    StateWrite,
};

use crate::accounts::state_ext::StateReadExt;

/// The ICS20 transfer handler.
/// See https://github.com/cosmos/ibc/blob/main/spec/app/ics-020-fungible-token-transfer/README.md
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
        .await
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

    let packet_data = FungibleTokenPacketData::decode(data)?;
    let denom = packet_data.denom;

    if is_source(source_port, &source_channel, &denom) {
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

fn is_source(source_port: &PortId, source_channel: &ChannelId, denom: &str) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");
    denom.starts_with(&prefix)
}

#[async_trait::async_trait]
impl AppHandlerExecute for Ics20Transfer {
    async fn chan_open_init_execute<S: StateWrite>(state: S, msg: &MsgChannelOpenInit) {}

    async fn chan_open_try_execute<S: StateWrite>(state: S, msg: &MsgChannelOpenTry) {}

    async fn chan_open_ack_execute<S: StateWrite>(state: S, msg: &MsgChannelOpenAck) {}

    async fn chan_open_confirm_execute<S: StateWrite>(state: S, msg: &MsgChannelOpenConfirm) {}

    async fn chan_close_confirm_execute<S: StateWrite>(state: S, msg: &MsgChannelCloseConfirm) {}

    async fn chan_close_init_execute<S: StateWrite>(state: S, msg: &MsgChannelCloseInit) {}

    async fn recv_packet_execute<S: StateWrite>(state: S, msg: &MsgRecvPacket) {}

    async fn timeout_packet_execute<S: StateWrite>(state: S, msg: &MsgTimeout) {}

    async fn acknowledge_packet_execute<S: StateWrite>(state: S, msg: &MsgAcknowledgement) {}
}

#[async_trait::async_trait]
impl AppHandler for Ics20Transfer {}
