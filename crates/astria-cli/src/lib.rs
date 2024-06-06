use astria_core::primitive::v1::{
    Address,
    AddressError,
};

pub mod cli;
pub mod commands;
pub mod types;

const ADDRESS_PREFIX: &str = "astria";

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
