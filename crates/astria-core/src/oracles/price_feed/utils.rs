use indexmap::IndexMap;
use prost::Message as _;
use tendermint::abci::types::ExtendedCommitInfo;

use crate::{
    generated::price_feed::abci::v2::OracleVoteExtension as RawOracleVoteExtension,
    oracles::price_feed::{
        abci::v2::{
            OracleVoteExtension,
            OracleVoteExtensionError,
        },
        types::v2::{
            CurrencyPairId,
            Price,
        },
    },
    protocol::price_feed::v1::CurrencyPairInfo,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn decode_error(err: prost::DecodeError) -> Self {
        Self(ErrorKind::DecodeError(err))
    }

    fn invalid_oracle_vote_extension(err: OracleVoteExtensionError) -> Self {
        Self(ErrorKind::InvalidOracleVoteExtension(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("failed to decode price feed oracle vote extension")]
    DecodeError(#[from] prost::DecodeError),
    #[error("failed to convert raw price feed oracle vote extension to native")]
    InvalidOracleVoteExtension(#[from] OracleVoteExtensionError),
}

/// Calculates the median price for each currency pair from the given vote extensions in the
/// `extended_commit_info`.
///
/// # Errors
///
/// - if any of the vote extensions cannot be decoded into a protobuf `OracleVoteExtension` message
/// - if any of the vote extensions cannot be converted from prootobuf to native
///   `OracleVoteExtension`
pub fn calculate_prices_from_vote_extensions(
    extended_commit_info: &ExtendedCommitInfo,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPairInfo>,
) -> Result<Vec<crate::sequencerblock::v1::block::Price>, Error> {
    let votes = extended_commit_info
        .votes
        .iter()
        .map(|vote| {
            let raw = RawOracleVoteExtension::decode(vote.vote_extension.as_ref())
                .map_err(Error::decode_error)?;
            OracleVoteExtension::try_from_raw(raw).map_err(Error::invalid_oracle_vote_extension)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let prices = aggregate_oracle_votes(votes, id_to_currency_pair).collect();
    Ok(prices)
}

fn aggregate_oracle_votes(
    votes: Vec<OracleVoteExtension>,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPairInfo>,
) -> impl Iterator<Item = crate::sequencerblock::v1::block::Price> {
    // validators are not weighted right now, so we just take the median price for each currency
    // pair
    //
    // skip uses a stake-weighted median: https://github.com/skip-mev/connect/blob/19a916122110cfd0e98d93978107d7ada1586918/pkg/math/voteweighted/voteweighted.go#L59
    // we can implement this later, when we have stake weighting.
    let mut currency_pair_to_price_list = IndexMap::new();
    for vote in votes {
        for (id, price) in vote.prices {
            let Some(currency_pair) = id_to_currency_pair.get(&id).cloned() else {
                // it's possible for a vote to contain some currency pair ID that didn't exist
                // in state. this probably shouldn't happen if validators are running the right
                // code, but it doesn't invalidate their entire vote extension, so
                // it's kept in the block anyways.
                continue;
            };
            currency_pair_to_price_list
                .entry(currency_pair)
                .and_modify(|prices: &mut Vec<Price>| prices.push(price))
                .or_insert(vec![price]);
        }
    }

    currency_pair_to_price_list
        .into_iter()
        .filter_map(|(currency_pair_info, price_list)| {
            median(price_list).map(|median| {
                crate::sequencerblock::v1::block::Price::new(
                    currency_pair_info.currency_pair,
                    median,
                    currency_pair_info.decimals,
                )
            })
        })
}

/// Returns the median value from `price_list`, or an error if the list is empty.
fn median(mut price_list: Vec<Price>) -> Option<Price> {
    price_list.sort_unstable();
    let midpoint = price_list
        .len()
        .checked_div(2)
        .expect("can't fail as divisor is not zero");
    if price_list.len() % 2 == 1 {
        return Some(
            price_list
                .get(midpoint)
                .copied()
                .expect("`midpoint` is a valid index"),
        );
    }

    let Some(lower_index) = midpoint.checked_sub(1) else {
        // We can only get here if `price_list` is empty; this is not supported, so return None.
        return None;
    };

    // `price_list.len()` >= 2 if we got to here, meaning `midpoint` and `lower_index` must both be
    // valid indices of `price_list`.
    let higher_price = price_list
        .get(midpoint)
        .expect("`midpoint` is a valid index");
    let lower_price = price_list
        .get(lower_index)
        .expect("`lower_index` is a valid index");
    // Avoid overflow by halving both values first.
    let half_high = higher_price
        .checked_div(2)
        .expect("can't fail as divisor is not zero");
    let half_low = lower_price
        .checked_div(2)
        .expect("can't fail as divisor is not zero");
    let sum = half_high
        .checked_add(half_low)
        .expect("can't fail as both operands are <= MAX/2");
    // If `higher_price` and `lower_price` are both odd, we rounded down twice when halving them,
    // so add 1 to the sum.
    let median = if higher_price.get() % 2 == 1 && lower_price.get() % 2 == 1 {
        sum.checked_add(Price::new(1))
            .expect("can't fail as we rounded down twice while halving the prices")
    } else {
        sum
    };
    Some(median)
}

#[cfg(test)]
mod test {
    use indexmap::indexmap;

    use super::*;

    fn get_id_to_currency_pair_mapping() -> IndexMap<CurrencyPairId, CurrencyPairInfo> {
        indexmap! {
            CurrencyPairId::new(0) => CurrencyPairInfo {
                currency_pair: "ETH/USD".parse().unwrap(),
                decimals: 0,
            },
            CurrencyPairId::new(1) => CurrencyPairInfo {
                currency_pair: "BTC/USD".parse().unwrap(),
                decimals: 0,
            },
            CurrencyPairId::new(2) => CurrencyPairInfo {
                currency_pair: "TIA/USD".parse().unwrap(),
                decimals: 0,
            },
        }
    }

    fn oracle_vote_extension<I: IntoIterator<Item = i128>>(prices: I) -> OracleVoteExtension {
        OracleVoteExtension {
            prices: prices
                .into_iter()
                .enumerate()
                .map(|(index, price)| (CurrencyPairId::new(index as u64), Price::new(price)))
                .collect(),
        }
    }

    #[test]
    fn aggregate_oracle_votes_ok() {
        let votes = vec![
            oracle_vote_extension([9, 19, 29]),
            oracle_vote_extension([10, 20, 30]),
            oracle_vote_extension([11, 21, 31]),
        ];
        let id_to_currency_pairs = get_id_to_currency_pair_mapping();
        let mut aggregated_prices = aggregate_oracle_votes(votes, &id_to_currency_pairs);
        assert_eq!(Price::new(10), aggregated_prices.next().unwrap().price());
        assert_eq!(Price::new(20), aggregated_prices.next().unwrap().price());
        assert_eq!(Price::new(30), aggregated_prices.next().unwrap().price());
        assert!(aggregated_prices.next().is_none());
    }

    #[test]
    fn aggregate_oracle_votes_should_skip_unknown_pairs() {
        // Last two entries in each vote should be ignored as we haven't stored state for them in
        // storage, so there is no mapping of their `CurrencyPairId` to `CurrencyPair`.
        let votes = vec![
            oracle_vote_extension([9, 19, 29, 39, 49]),
            oracle_vote_extension([10, 20, 30, 40, 50]),
            oracle_vote_extension([11, 21, 31, 41, 51]),
        ];
        let id_to_currency_pairs = get_id_to_currency_pair_mapping();
        let mut aggregated_prices = aggregate_oracle_votes(votes, &id_to_currency_pairs);
        assert_eq!(Price::new(10), aggregated_prices.next().unwrap().price());
        assert_eq!(Price::new(20), aggregated_prices.next().unwrap().price());
        assert_eq!(Price::new(30), aggregated_prices.next().unwrap().price());
        assert!(aggregated_prices.next().is_none());
    }

    #[test]
    fn should_calculate_median() {
        fn prices<I: IntoIterator<Item = i128>>(prices: I) -> Vec<Price> {
            prices.into_iter().map(Price::new).collect()
        }

        // Empty set should return None.
        assert!(median(vec![]).is_none(),);

        // Should handle a set with 1 entry.
        assert_eq!(1, median(prices([1])).unwrap().get());

        // Should handle a set with 2 entries.
        assert_eq!(15, median(prices([20, 10])).unwrap().get());

        // Should handle a larger set with odd number of entries.
        assert_eq!(10, median(prices([21, 22, 23, 1, 2, 10, 3])).unwrap().get());

        // Should handle a larger set with even number of entries.
        assert_eq!(12, median(prices([21, 22, 23, 1, 2, 3])).unwrap().get());

        // Should round down if required.
        assert_eq!(17, median(prices([10, 15, 20, 25])).unwrap().get());

        // Should handle large values in a set with odd number of entries.
        assert_eq!(
            i128::MAX,
            median(prices([i128::MAX, i128::MAX, 1])).unwrap().get()
        );

        // Should handle large values in a set with even number of entries.
        assert_eq!(
            i128::MAX - 1,
            median(prices([i128::MAX, i128::MAX, i128::MAX - 1, i128::MAX - 1]))
                .unwrap()
                .get()
        );
    }
}
