#[cfg(any(feature = "http", feature = "websocket"))]
pub mod extension_trait;

#[cfg(not(any(feature = "http", feature = "websocket")))]
compile_error!("at least one of the `http` or `websocket` features must be enabled");

#[cfg(any(feature = "http", feature = "websocket"))]
pub use __feature_gated_exports::*;
pub use astria_core::sequencer::v1::{
    Address,
    BalanceResponse,
    NonceResponse,
    SequencerBlock,
    SignedTransaction,
};
pub use tendermint;
pub use tendermint_proto;
pub use tendermint_rpc;
#[cfg(feature = "http")]
pub use tendermint_rpc::HttpClient;
#[cfg(feature = "websocket")]
pub use tendermint_rpc::WebSocketClient;
#[cfg(any(feature = "http", feature = "websocket"))]
mod __feature_gated_exports {
    pub use tendermint_rpc::{
        Client,
        SubscriptionClient,
    };

    pub use crate::extension_trait::{
        NewBlockStreamError,
        SequencerClientExt,
        SequencerSubscriptionClientExt,
    };
}

#[cfg(test)]
mod tests;
