//! # Astria Conductor
//! The Astria conductor connects the shared sequencer layer and the execution layer.
//! When a block is received from the sequencer layer, the conductor pushes it to the execution
//! layer. There are two ways for a block to be received:
//! - via the gossip network
//! - via the data availability layer
//! In the first case, the block is pushed to the execution layer, executed, and added to the
//! blockchain. It's marked as a soft commitment; the block is not finalized until it's received
//! from the data availability layer. In the second case, the execution layer is notified to mark
//! the block as finalized.
pub mod block_verifier;
pub mod config;
pub mod driver;
pub(crate) mod execution_client;
pub(crate) mod executor;
pub mod reader;
pub(crate) mod types;

pub use telemetry;

mod private {
    #[allow(unreachable_pub)]
    pub trait Sealed {}
}
