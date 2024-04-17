pub mod client;
pub mod metrics_init;
pub mod submission;

pub use astria_core::sequencerblock::v1alpha1::{
    CelestiaRollupBlob,
    CelestiaSequencerBlob,
};
pub use celestia_rpc;
pub use celestia_tendermint;
pub use celestia_types;
use celestia_types::nmt::{
    Namespace,
    NS_ID_V0_SIZE,
};
pub use client::CelestiaClientExt;
pub use jsonrpsee;

pub fn is_blob_not_found<T: IsBlobNotFound>(err: &T) -> bool {
    err.is_blob_not_found()
}

pub trait IsBlobNotFound {
    fn is_blob_not_found(&self) -> bool;
}

impl IsBlobNotFound for jsonrpsee::core::Error {
    fn is_blob_not_found(&self) -> bool {
        let jsonrpsee::core::Error::Call(error) = self else {
            return false;
        };
        error.code() == 1 && error.message().contains("blob: not found")
    }
}

#[must_use = "a celestia namespace must be used in order to be useful"]
pub const fn celestia_namespace_v0_from_array<const N: usize>(bytes: [u8; N]) -> Namespace {
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(
        10 == NS_ID_V0_SIZE,
        "verify that the celestia v0 namespace was changed from 10 bytes"
    );
    let first_10_bytes = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        bytes[9],
    ];
    Namespace::const_v0(first_10_bytes)
}

#[must_use = "a celestia namespace must be used in order to be useful"]
pub const fn celestia_namespace_v0_from_rollup_id(
    rollup_id: astria_core::sequencer::v1::RollupId,
) -> Namespace {
    celestia_namespace_v0_from_array(rollup_id.get())
}

#[must_use = "a celestia namespace must be used in order to be useful"]
pub fn celestia_namespace_v0_from_str(chain_id: &str) -> Namespace {
    use sha2::{
        Digest as _,
        Sha256,
    };
    celestia_namespace_v0_from_array(Sha256::digest(chain_id.as_bytes()).into())
}
