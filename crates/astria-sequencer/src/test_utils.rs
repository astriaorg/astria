use std::str::FromStr as _;

use astria_core::{
    oracles::price_feed::{
        market_map::v2::Ticker,
        types::v2::{
            Base,
            CurrencyPair,
            Quote,
        },
    },
    primitive::v1::{
        Address,
        Bech32,
    },
    protocol::transaction::v1::action::RollupDataSubmission,
};

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
    let fees = state
        .get_fees::<RollupDataSubmission>()
        .await
        .expect("should not error fetching rollup data submission fees")
        .expect("rollup data submission fees should be stored");
    fees.base()
        .checked_add(
            fees.multiplier()
                .checked_mul(
                    data.len()
                        .try_into()
                        .expect("a usize should always convert to a u128"),
                )
                .expect("fee multiplication should not overflow"),
        )
        .expect("fee addition should not overflow")
}

pub(crate) fn borsh_then_hex<T: borsh::BorshSerialize>(item: &T) -> String {
    hex::encode(borsh::to_vec(item).unwrap())
}

pub(crate) fn example_ticker_with_metadata(metadata: String) -> Ticker {
    Ticker {
        currency_pair: CurrencyPair::from_parts(
            Base::from_str("BTC").unwrap(),
            Quote::from_str("USD").unwrap(),
        ),
        decimals: 2,
        min_provider_count: 2,
        enabled: true,
        metadata_json: metadata,
    }
}

pub(crate) fn example_ticker_from_currency_pair(
    base: &str,
    quote: &str,
    metadata: String,
) -> Ticker {
    Ticker {
        currency_pair: CurrencyPair::from_parts(
            Base::from_str(base).unwrap(),
            Quote::from_str(quote).unwrap(),
        ),
        decimals: 2,
        min_provider_count: 2,
        enabled: true,
        metadata_json: metadata,
    }
}
