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

#[derive(Clone)]
pub(crate) struct Ics20Transfer {}
