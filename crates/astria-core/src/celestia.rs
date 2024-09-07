pub use celestia_types::nmt::Namespace;
/// Constructs a [`celestia_types::nmt::Namespace`] from the first 10 bytes of a byte slice.
///
/// # Panics
/// Panics if `bytes` contains less then 10 bytes.
#[must_use = "a celestia namespace must be used in order to be useful"]
pub const fn namespace_v0_from_first_10_bytes(bytes: &[u8]) -> Namespace {
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(
        10 == celestia_types::nmt::NS_ID_V0_SIZE,
        "verify that the celestia v0 namespace was changed from 10 bytes"
    );
    let first_10_bytes = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        bytes[9],
    ];
    Namespace::const_v0(first_10_bytes)
}

/// Constructs a [`celestia_types::nmt::Namespace`] from the first
/// 10 bytes of [`crate::primitive::v1::RollupId`].
#[must_use = "a celestia namespace must be used in order to be useful"]
pub const fn namespace_v0_from_rollup_id(rollup_id: crate::primitive::v1::RollupId) -> Namespace {
    namespace_v0_from_first_10_bytes(rollup_id.get())
}

/// Constructs a [`celestia_types::nmt::Namespace`] from the first 10 bytes of the sha256 hash of
/// `bytes`.
#[must_use = "a celestia namespace must be used in order to be useful"]
pub fn namespace_v0_from_sha256_of_bytes<T: AsRef<[u8]>>(bytes: T) -> Namespace {
    use sha2::{
        Digest as _,
        Sha256,
    };
    namespace_v0_from_first_10_bytes(&Sha256::digest(bytes))
}
