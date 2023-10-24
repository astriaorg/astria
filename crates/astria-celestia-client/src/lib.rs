pub mod blob_space;
pub mod client;

pub use blob_space::{
    RollupNamespaceData,
    SequencerNamespaceData,
};
pub use celestia_rpc;
pub use celestia_tendermint;
pub use celestia_types;
use celestia_types::nmt::Namespace;
pub use client::CelestiaClientExt;
pub use jsonrpsee;

pub const SEQUENCER_NAMESPACE: Namespace = Namespace::const_v0(*b"astriasequ");
