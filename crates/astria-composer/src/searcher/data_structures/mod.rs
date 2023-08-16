use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

// Module Declarations
pub(super) mod rollup_tx;
pub(super) mod streaming_client;

pub(super) type RollupChainId = String;
pub(super) type RollupTxExt = (rollup_tx::RollupTx, RollupChainId);

/// An Actor channel is a common channel type that is used to communicate
/// between the different components of the searcher, each of which behave
/// like an Actor in the Actor pattern
pub(super) struct ActorChannel<I, O> {
    pub incoming: Option<UnboundedReceiver<I>>,
    pub outgoing: Option<UnboundedSender<O>>,
}
