use prost::Message as _;
use tendermint::abci::types::ExtendedCommitInfo;

use crate::{
    connect::abci::v2::OracleVoteExtension,
    generated::connect::abci::v2::OracleVoteExtension as RawOracleVoteExtension,
    sequencerblock::v1::block::RollupData,
};

pub fn parse_extended_commit_info_into_oracle_data(extended_commit_info: Vec<u8>) -> RollupData {
    let extended_commit_info =
        tendermint_proto::abci::ExtendedCommitInfo::decode(&extended_commit_info)
            .expect("failed to decode extended commit info");
    RollupData::OracleData(oracle_data)
}

pub async fn calculate_prices_from_vote_extensions(
    extended_commit_info: ExtendedCommitInfo,
    timestamp: Timestamp,
    height: u64,
) -> Result<()> {
    let votes = extended_commit_info
        .votes
        .iter()
        .map(|vote| {
            let raw = RawOracleVoteExtension::decode(vote.vote_extension.clone())
                .wrap_err("failed to decode oracle vote extension")?;
            OracleVoteExtension::try_from_raw(raw)
                .wrap_err("failed to validate oracle vote extension")
        })
        .collect::<Result<Vec<_>>>()
        .wrap_err("failed to extract oracle vote extension from extended commit info")?;

    let prices = aggregate_oracle_votes(state, votes)
        .await
        .wrap_err("failed to aggregate oracle votes")?;

    for (currency_pair, price) in prices {
        let price = QuotePrice {
            price,
            block_timestamp: astria_core::Timestamp {
                seconds: timestamp.seconds,
                nanos: timestamp.nanos,
            },
            block_height: height,
        };

        state
            .put_price_for_currency_pair(currency_pair, price)
            .await
            .wrap_err("failed to put price")?;
    }

    Ok(())
}

async fn aggregate_oracle_votes<S: StateReadExt>(
    state: &S,
    votes: Vec<OracleVoteExtension>,
) -> Result<HashMap<CurrencyPair, Price>> {
    // validators are not weighted right now, so we just take the median price for each currency
    // pair
    //
    // skip uses a stake-weighted median: https://github.com/skip-mev/connect/blob/19a916122110cfd0e98d93978107d7ada1586918/pkg/math/voteweighted/voteweighted.go#L59
    // we can implement this later, when we have stake weighting.
    let mut currency_pair_to_price_list = HashMap::new();
    for vote in votes {
        for (id, price) in vote.prices {
            let Some(currency_pair) = DefaultCurrencyPairStrategy::from_id(state, id)
                .await
                .wrap_err("failed to get currency pair from id")?
            else {
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
        prices.insert(currency_pair, median_price);
    }

    Ok(prices)
}
