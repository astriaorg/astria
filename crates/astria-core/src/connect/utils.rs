use std::collections::HashMap;

use indexmap::IndexMap;
use prost::Message as _;
use tendermint::abci::types::ExtendedCommitInfo;

use crate::{
    connect::{
        abci::v2::{
            OracleVoteExtension,
            OracleVoteExtensionError,
        },
        types::v2::{
            CurrencyPair,
            CurrencyPairId,
            Price,
        },
    },
    generated::connect::abci::v2::OracleVoteExtension as RawOracleVoteExtension,
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
    #[error("failed to decode oracle vote extension")]
    DecodeError(#[from] prost::DecodeError),
    #[error("failed to convert raw oracle vote extension to native")]
    InvalidOracleVoteExtension(#[from] OracleVoteExtensionError),
}

pub async fn calculate_prices_from_vote_extensions(
    extended_commit_info: ExtendedCommitInfo,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPair>,
) -> Result<HashMap<CurrencyPair, Price>, Error> {
    let votes = extended_commit_info
        .votes
        .iter()
        .map(|vote| {
            let raw = RawOracleVoteExtension::decode(vote.vote_extension.clone())
                .map_err(Error::decode_error)?;
            OracleVoteExtension::try_from_raw(raw).map_err(Error::invalid_oracle_vote_extension)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let prices = aggregate_oracle_votes(votes, id_to_currency_pair);
    Ok(prices)
}

fn aggregate_oracle_votes(
    votes: Vec<OracleVoteExtension>,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPair>,
) -> HashMap<CurrencyPair, Price> {
    // validators are not weighted right now, so we just take the median price for each currency
    // pair
    //
    // skip uses a stake-weighted median: https://github.com/skip-mev/connect/blob/19a916122110cfd0e98d93978107d7ada1586918/pkg/math/voteweighted/voteweighted.go#L59
    // we can implement this later, when we have stake weighting.
    let mut currency_pair_to_price_list = HashMap::new();
    for vote in votes {
        for (id, price) in vote.prices {
            let Some(currency_pair) = id_to_currency_pair.get(&id) else {
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

    let mut prices = HashMap::new();
    for (currency_pair, mut price_list) in currency_pair_to_price_list {
        price_list.sort_unstable();
        let midpoint = price_list
            .len()
            .checked_div(2)
            .expect("has a result because RHS is not 0");
        let median_price = if price_list.len() % 2 == 0 {
            'median_from_even: {
                let Some(left) = price_list.get(midpoint) else {
                    break 'median_from_even None;
                };
                let Some(right_idx) = midpoint.checked_add(1) else {
                    break 'median_from_even None;
                };
                let Some(right) = price_list.get(right_idx).copied() else {
                    break 'median_from_even None;
                };
                left.checked_add(right).and_then(|sum| sum.checked_div(2))
            }
        } else {
            price_list.get(midpoint).copied()
        }
        .unwrap_or_else(|| Price::new(0));
        prices.insert(currency_pair.clone(), median_price);
    }

    prices
}
