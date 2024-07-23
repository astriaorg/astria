use astria_core::crypto::{
    SigningKey,
    VerificationKey,
};

pub(crate) fn verification_key(seed: u64) -> VerificationKey {
    use rand::SeedableRng as _;
    let rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
    let signing_key = SigningKey::new(rng);
    signing_key.verification_key()
}
