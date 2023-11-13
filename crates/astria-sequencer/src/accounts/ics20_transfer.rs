use anyhow::Result;
use ibc_types::core::channel::msgs::{
    MsgAcknowledgement,
    MsgChannelCloseConfirm,
    MsgChannelCloseInit,
    MsgChannelOpenAck,
    MsgChannelOpenConfirm,
    MsgChannelOpenInit,
    MsgChannelOpenTry,
    MsgRecvPacket,
    MsgTimeout,
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
use penumbra_storage::{
    StateRead,
    StateWrite,
};

#[derive(Clone)]
pub(crate) struct Ics20Transfer {}

#[async_trait::async_trait]
impl AppHandlerCheck for Ics20Transfer {
    async fn chan_open_init_check<S: StateRead>(state: S, msg: &MsgChannelOpenInit) -> Result<()> {
        todo!()
    }

    async fn chan_open_try_check<S: StateRead>(state: S, msg: &MsgChannelOpenTry) -> Result<()> {
        todo!()
    }

    async fn chan_open_ack_check<S: StateRead>(state: S, msg: &MsgChannelOpenAck) -> Result<()> {
        todo!()
    }

    async fn chan_open_confirm_check<S: StateRead>(
        state: S,
        msg: &MsgChannelOpenConfirm,
    ) -> Result<()> {
        todo!()
    }

    async fn chan_close_confirm_check<S: StateRead>(
        state: S,
        msg: &MsgChannelCloseConfirm,
    ) -> Result<()> {
        todo!()
    }

    async fn chan_close_init_check<S: StateRead>(
        state: S,
        msg: &MsgChannelCloseInit,
    ) -> Result<()> {
        todo!()
    }

    async fn recv_packet_check<S: StateRead>(state: S, msg: &MsgRecvPacket) -> Result<()> {
        todo!()
    }

    async fn timeout_packet_check<S: StateRead>(state: S, msg: &MsgTimeout) -> Result<()> {
        todo!()
    }

    async fn acknowledge_packet_check<S: StateRead>(
        state: S,
        msg: &MsgAcknowledgement,
    ) -> Result<()> {
        todo!()
    }
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
