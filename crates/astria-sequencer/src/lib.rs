pub(crate) mod accounts;
pub(crate) mod app;
pub(crate) mod app_hash;
pub(crate) mod component;
pub mod config;
pub(crate) mod crypto;
pub(crate) mod genesis;
mod sequencer;
pub(crate) mod service;
pub(crate) mod state_ext;
pub mod telemetry;
pub(crate) mod transaction;

pub use config::Config;
pub use sequencer::Sequencer;

pub(crate) fn hash(s: &[u8]) -> Vec<u8> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(s);
    hasher.finalize().to_vec()
}
