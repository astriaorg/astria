use astria_core::primitive::v1::{
    asset::TracePrefixed,
    Address,
};

pub(crate) const ASTRIA_PREFIX: &str = "astria";
pub(crate) const ASTRIA_COMPAT_PREFIX: &str = "astriacompat";

pub(crate) fn astria_address_from_hex_string(s: &str) -> Address {
    let bytes = hex::decode(s).unwrap();
    Address::builder()
        .prefix(ASTRIA_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}

pub(crate) fn nria() -> TracePrefixed {
    "nria".parse().unwrap()
}

#[cfg(test)]
pub(crate) fn verification_key(seed: u64) -> astria_core::crypto::VerificationKey {
    use rand::SeedableRng as _;
    let rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
    let signing_key = astria_core::crypto::SigningKey::new(rng);
    signing_key.verification_key()
}

#[cfg(test)]
#[track_caller]
pub(crate) fn assert_eyre_error(error: &astria_eyre::eyre::Error, expected: &'_ str) {
    let msg = format!("{error:?}");
    assert!(
        msg.contains(expected),
        "error contained different message\n\texpected: {expected}\n\tfull_error: {msg}",
    );
}

pub(crate) fn astria_address(bytes: &[u8]) -> Address {
    Address::builder()
        .prefix(ASTRIA_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}
