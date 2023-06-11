pub mod accounts;
pub mod app;
pub mod component;
pub mod consensus;
pub mod crypto;
pub mod info;
pub mod mempool;
pub mod sequencer;
pub mod snapshot;
pub mod state_ext;
pub mod telemetry;
pub mod transaction;

pub(crate) fn hash(s: &[u8]) -> Vec<u8> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(s);
    hasher.finalize().to_vec()
}
