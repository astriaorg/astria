//! The application hash is used to verify the state of the application state.
//!
//! Modelled after [`penumbra_chain::component::AppHash`].
//!
//! [`penumbra_chain::component::AppHash`]: https://github.com/penumbra-zone/penumbra/blob/22cbaffe5843f3e1be86ac1a27591db01d0368b4/crates/core/component/chain/src/component.rs
use sha2::{
    Digest as _,
    Sha256,
};
use storage::RootHash;

const APPHASH_DOMSEP: &str = "AstriaAppHash";

/// The application hash, used to verify the application state.
///
/// The app hash of astria's state is defined as
/// `SHA256("AstriaAppHash" || jmt.root_hash())`.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct AppHash(pub(crate) [u8; 32]);

impl std::fmt::Debug for AppHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AppHash")
            .field(&hex::encode(self.0))
            .finish()
    }
}

impl From<RootHash> for AppHash {
    fn from(root_hash: RootHash) -> Self {
        let mut h = Sha256::new();
        h.update(APPHASH_DOMSEP);
        h.update(root_hash.0);
        Self(h.finalize().into())
    }
}
