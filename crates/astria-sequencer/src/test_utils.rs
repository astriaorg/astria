use astria_core::primitive::v1::{
    Address,
    Bech32,
};
#[cfg(test)]
use astria_core::protocol::fees::v1::RollupDataSubmissionFeeComponents;

use crate::benchmark_and_test_utils::ASTRIA_COMPAT_PREFIX;

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
