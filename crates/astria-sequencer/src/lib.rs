pub(crate) mod accounts;
mod api_state_ext;
pub(crate) mod app;
pub(crate) mod asset;
pub(crate) mod authority;
pub(crate) mod bridge;
mod build_info;
pub(crate) mod component;
pub mod config;
pub(crate) mod fee_asset_change;
pub(crate) mod genesis;
pub(crate) mod grpc;
pub(crate) mod ibc;
mod mempool;
pub mod metrics_init;
pub(crate) mod proposal;
pub(crate) mod sequence;
mod sequencer;
pub(crate) mod service;
pub(crate) mod state_ext;
pub(crate) mod transaction;
mod utils;

use astria_core::primitive::v1::{
    Address,
    AddressError,
};
pub use build_info::BUILD_INFO;
pub use config::Config;
pub(crate) use config::ADDRESS_PREFIX;
pub use sequencer::Sequencer;
pub use telemetry;

/// Constructs an [`Address`] prefixed by `"astria"`.
pub(crate) fn astria_address(array: [u8; astria_core::primitive::v1::ADDRESS_LEN]) -> Address {
    Address::builder()
        .array(array)
        .prefix(ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}

/// Tries to construct an [`Address`] prefixed by `"astria"` from a byte slice.
///
/// # Errors
/// Fails if the slice does not contain 20 bytes.
pub(crate) fn try_astria_address(slice: &[u8]) -> Result<Address, AddressError> {
    Address::builder()
        .slice(slice)
        .prefix(ADDRESS_PREFIX)
        .try_build()
}
