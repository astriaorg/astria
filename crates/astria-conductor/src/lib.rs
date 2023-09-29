//! # Astria Conductor
//! The Astria conductor connects the shared sequencer layer and the execution layer.
//! When a block is received from the sequencer layer, the conductor pushes it to the execution
//! layer. There are two ways for a block to be received:
//! - pushed from the shared-sequencer
//! - via the data availability layer
//! In the first case, the block is pushed to the execution layer, executed, and added to the
//! blockchain. It's marked as a soft commitment; the block is not regarded as finalized on the
//! execution layer until it's received from the data availability layer. In the second case, the
//! execution layer is notified to mark the block as finalized.
pub(crate) mod block_verifier;
pub mod conductor;
pub mod config;
pub(crate) mod data_availability;
pub(crate) mod execution_client;
pub(crate) mod executor;
pub(crate) mod sequencer;
pub(crate) mod types;

pub use config::Config;
