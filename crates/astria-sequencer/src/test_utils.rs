use astria_core::primitive::v1::{
    asset::TracePrefixed,
    Address,
    Bech32,
};
#[cfg(test)]
use astria_core::protocol::fees::v1::RollupDataSubmissionFeeComponents;

pub(crate) const ASTRIA_PREFIX: &str = "astria";
pub(crate) const ASTRIA_COMPAT_PREFIX: &str = "astriacompat";

pub(crate) fn astria_address(bytes: &[u8]) -> Address {
    Address::builder()
        .prefix(ASTRIA_PREFIX)
        .slice(bytes)
        .try_build()
        .unwrap()
}

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "allow is only necessary when benchmark isn't enabled"
)]
#[cfg_attr(feature = "benchmark", allow(dead_code))]
pub(crate) fn astria_compat_address(bytes: &[u8]) -> Address<Bech32> {
    Address::builder()
        .prefix(ASTRIA_COMPAT_PREFIX)
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

#[cfg(test)]
pub(crate) fn verification_key(seed: u64) -> astria_core::crypto::VerificationKey {
    use rand::SeedableRng as _;
    let rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
    let signing_key = astria_core::crypto::SigningKey::new(rng);
    signing_key.verification_key()
}

#[cfg(test)]
#[track_caller]
pub(crate) fn assert_eyre_error(error: &astria_eyre::eyre::Error, expected: &'static str) {
    let msg = error.to_string();
    assert!(
        msg.contains(expected),
        "error contained different message\n\texpected: {expected}\n\tfull_error: {msg}",
    );
}

/// Calculates the fee for a sequence `Action` based on the length of the `data`.
#[cfg(test)]
pub(crate) async fn calculate_rollup_data_submission_fee_from_state<
    S: crate::fees::StateReadExt,
>(
    data: &[u8],
    state: &S,
) -> u128 {
    let RollupDataSubmissionFeeComponents {
        base,
        multiplier,
    } = state
        .get_rollup_data_submission_fees()
        .await
        .expect("should not error fetching rollup data submission fees")
        .expect("rollup data submission fees should be stored");
    base.checked_add(
        multiplier
            .checked_mul(
                data.len()
                    .try_into()
                    .expect("a usize should always convert to a u128"),
            )
            .expect("fee multiplication should not overflow"),
    )
    .expect("fee addition should not overflow")
}
