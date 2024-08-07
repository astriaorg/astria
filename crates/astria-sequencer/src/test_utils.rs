use astria_core::{
    crypto::{
        SigningKey,
        VerificationKey,
    },
    primitive::v1::{
        asset::TracePrefixed,
        Address,
    },
};

pub(crate) const ASTRIA_PREFIX: &str = "astria";

pub(crate) fn astria_address(bytes: &[u8]) -> Address {
    Address::builder()
        .prefix(ASTRIA_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}

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

pub(crate) fn verification_key(seed: u64) -> VerificationKey {
    use rand::SeedableRng as _;
    let rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
    let signing_key = SigningKey::new(rng);
    signing_key.verification_key()
}
