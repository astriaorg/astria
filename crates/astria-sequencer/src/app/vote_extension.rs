use std::collections::HashMap;

use anyhow::{
    ensure,
    Context as _,
};
use astria_core::{
    crypto::{
        Signature,
        VerificationKey,
    },
    generated::slinky::{
        abci::v1::OracleVoteExtension as RawOracleVoteExtension,
        service::v1::{
            oracle_client::OracleClient,
            QueryPricesRequest,
            QueryPricesResponse,
        },
    },
    slinky::{
        abci::v1::OracleVoteExtension,
        oracle::v1::QuotePrice,
        types::v1::CurrencyPair,
    },
};
use prost::Message as _;
use tendermint::{
    abci,
    abci::types::{
        BlockSignatureInfo::Flag,
        CommitInfo,
        ExtendedCommitInfo,
    },
};
use tendermint_proto::google::protobuf::Timestamp;
use tonic::transport::Channel;
use tracing::debug;

use crate::{
    authority::state_ext::StateReadExt as _,
    slinky::oracle::{
        currency_pair_strategy::DefaultCurrencyPairStrategy,
        state_ext::StateWriteExt,
    },
    state_ext::StateReadExt,
};

// https://github.com/skip-mev/slinky/blob/793b2e874d6e720bd288e82e782502e41cf06f8c/abci/types/constants.go#L6
const MAXIMUM_PRICE_BYTE_LEN: usize = 33;

pub(crate) struct Handler {
    // gRPC client for the slinky oracle sidecar.
    oracle_client: Option<OracleClient<Channel>>,
    oracle_client_timeout: tokio::time::Duration,
}

impl Handler {
    pub(crate) fn new(
        oracle_client: Option<OracleClient<Channel>>,
        oracle_client_timeout: u64,
    ) -> Self {
        Self {
            oracle_client,
            oracle_client_timeout: tokio::time::Duration::from_millis(oracle_client_timeout),
        }
    }

    pub(crate) async fn extend_vote<S: StateReadExt>(
        &mut self,
        state: &S,
    ) -> anyhow::Result<abci::response::ExtendVote> {
        tracing::info!("extending vote");
        let Some(oracle_client) = self.oracle_client.as_mut() else {
            // we allow validators to *not* use the oracle sidecar currently
            // however, if >1/3 of validators are not using the oracle, the prices will not update.
            return Ok(abci::response::ExtendVote {
                vote_extension: vec![].into(),
            });
        };

        tracing::info!("extending vote; getting prices from oracle sidecar");

        // if we fail to get prices within the timeout duration, we will return an empty vote
        // extension to ensure liveness.
        let prices = match tokio::time::timeout(
            self.oracle_client_timeout,
            oracle_client.prices(QueryPricesRequest {}),
        )
        .await
        {
            Ok(Ok(prices)) => prices.into_inner(),
            Ok(Err(e)) => {
                tracing::error!(
                    error = %e,
                    "failed to get prices from oracle sidecar"
                );
                return Ok(abci::response::ExtendVote {
                    vote_extension: vec![].into(),
                });
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "failed to get prices from oracle sidecar within timeout duration"
                );
                return Ok(abci::response::ExtendVote {
                    vote_extension: vec![].into(),
                });
            }
        };

        tracing::info!(prices = ?prices, "got prices from oracle sidecar; transforming prices");

        let oracle_vote_extension = transform_oracle_service_prices(state, prices)
            .await
            .context("failed to transform oracle service prices")?;

        Ok(abci::response::ExtendVote {
            vote_extension: oracle_vote_extension.into_raw().encode_to_vec().into(),
        })
    }

    pub(crate) async fn verify_vote_extension<S: StateReadExt>(
        &self,
        state: &S,
        vote: abci::request::VerifyVoteExtension,
        is_proposal_phase: bool,
    ) -> abci::response::VerifyVoteExtension {
        let oracle_vote_extension = match RawOracleVoteExtension::decode(vote.vote_extension) {
            Ok(oracle_vote_extension) => oracle_vote_extension.into(),
            Err(e) => {
                tracing::error!(error = %e, "failed to decode oracle vote extension");
                return abci::response::VerifyVoteExtension::Reject;
            }
        };
        match verify_vote_extension(state, oracle_vote_extension, is_proposal_phase).await {
            Ok(()) => abci::response::VerifyVoteExtension::Accept,
            Err(e) => {
                tracing::error!(error = %e, "failed to verify vote extension");
                abci::response::VerifyVoteExtension::Reject
            }
        }
    }
}

// see https://github.com/skip-mev/slinky/blob/5b07f91d6c0110e617efda3f298f147a31da0f25/abci/ve/utils.go#L24
pub(crate) async fn verify_vote_extension<S: StateReadExt>(
    state: &S,
    oracle_vote_extension: OracleVoteExtension,
    is_proposal_phase: bool,
) -> anyhow::Result<()> {
    let max_num_currency_pairs =
        DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state, is_proposal_phase)
            .await
            .context("failed to get max number of currency pairs")?;

    ensure!(
        oracle_vote_extension.prices.len() as u64 <= max_num_currency_pairs,
        "number of oracle vote extension prices exceeds max expected number of currency pairs"
    );

    for prices in oracle_vote_extension.prices.values() {
        ensure!(
            prices.len() <= MAXIMUM_PRICE_BYTE_LEN,
            "encoded price length exceeded {MAXIMUM_PRICE_BYTE_LEN}"
        );
    }

    Ok(())
}

// see https://github.com/skip-mev/slinky/blob/158cde8a4b774ac4eec5c6d1a2c16de6a8c6abb5/abci/ve/vote_extension.go#L290
async fn transform_oracle_service_prices<S: StateReadExt>(
    state: &S,
    prices: QueryPricesResponse,
) -> anyhow::Result<OracleVoteExtension> {
    let mut strategy_prices = HashMap::new();
    for (currency_pair_id, price_string) in prices.prices {
        let currency_pair = currency_pair_id
            .parse()
            .context("failed to parse currency pair")?;

        // TODO: how are the prices encoded into strings in the sidecar??
        let encoded_price = price_string.as_bytes();
        let price =
            DefaultCurrencyPairStrategy::get_decoded_price(state, &currency_pair, encoded_price)
                .await
                .context("failed to get decoded price")?;

        let id = DefaultCurrencyPairStrategy::id(state, &currency_pair)
            .await
            .context("failed to get id for currency pair")?;
        let encoded_price =
            DefaultCurrencyPairStrategy::get_encoded_price(state, &currency_pair, price).await;
        strategy_prices.insert(id, encoded_price);
    }

    Ok(OracleVoteExtension {
        prices: strategy_prices,
    })
}

pub(crate) struct ProposalHandler;

impl ProposalHandler {
    // called during prepare_proposal
    pub(crate) async fn prune_and_validate_extended_commit_info<S: StateReadExt>(
        state: &S,
        height: u64,
        mut extended_commit_info: ExtendedCommitInfo,
    ) -> anyhow::Result<ExtendedCommitInfo> {
        if height == 1 {
            // we're proposing block 1, so nothing to validate
            return Ok(extended_commit_info);
        }

        for vote in extended_commit_info.votes.iter_mut() {
            let oracle_vote_extension =
                RawOracleVoteExtension::decode(vote.vote_extension.clone())?.into();
            if let Err(e) = verify_vote_extension(state, oracle_vote_extension, true).await {
                debug!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    validator = crate::address::base_prefixed(vote.validator.address).to_string(),
                    "failed to verify vote extension; pruning from proposal"
                );
                vote.sig_info = Flag(tendermint::block::BlockIdFlag::Absent);
                vote.extension_signature = None;
                vote.vote_extension = vec![].into();
            }
        }
        validate_vote_extensions(state, height, &extended_commit_info)
            .await
            .context("failed to validate vote extensions in prepare_proposal")?;

        Ok(extended_commit_info)
    }

    // called during process_proposal
    pub(crate) async fn validate_extended_commit_info<S: StateReadExt>(
        state: &S,
        height: u64,
        last_commit: &CommitInfo,
        extended_commit_info: &ExtendedCommitInfo,
    ) -> anyhow::Result<()> {
        if height == 1 {
            // we're processing block 1, so nothing to validate (no last commit yet)
            return Ok(());
        }

        // inside process_proposal, we must validate the vote extensions proposed against the last
        // commit proposed
        validate_extended_commit_against_last_commit(last_commit, extended_commit_info)?;

        validate_vote_extensions(state, height, extended_commit_info)
            .await
            .context("failed to validate vote extensions in validate_extended_commit_info")?;
        Ok(())
    }
}

// see https://github.com/skip-mev/slinky/blob/5b07f91d6c0110e617efda3f298f147a31da0f25/abci/ve/utils.go#L111
async fn validate_vote_extensions<S: StateReadExt>(
    state: &S,
    height: u64,
    extended_commit_info: &ExtendedCommitInfo,
) -> anyhow::Result<()> {
    use tendermint_proto::v0_38::types::CanonicalVoteExtension;

    let chain_id = state
        .get_chain_id()
        .await
        .context("failed to get chain id")?;

    // total validator voting power
    let mut total_voting_power: u64 = 0;
    // the total voting power of all validators which submitted vote extensions
    let mut submitted_voting_power: u64 = 0;

    let validator_set = state
        .get_validator_set()
        .await
        .context("failed to get validator set")?;

    for vote in &extended_commit_info.votes {
        total_voting_power = total_voting_power.saturating_add(vote.validator.power.value());

        if vote.sig_info == Flag(tendermint::block::BlockIdFlag::Commit)
            && vote.extension_signature.is_none()
        {
            anyhow::bail!(
                "vote extension signature is missing for validator {}",
                crate::address::base_prefixed(vote.validator.address)
            );
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit)
            && vote.vote_extension.len() > 0
        {
            anyhow::bail!(
                "non-commit vote extension present for validator {}",
                crate::address::base_prefixed(vote.validator.address)
            );
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit)
            && vote.extension_signature.is_some()
        {
            anyhow::bail!(
                "non-commit extension signature present for validator {}",
                crate::address::base_prefixed(vote.validator.address)
            );
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit) {
            continue;
        }

        submitted_voting_power =
            submitted_voting_power.saturating_add(vote.validator.power.value());

        let pubkey = validator_set
            .get(
                &vote
                    .validator
                    .address
                    .to_vec()
                    .try_into()
                    .expect("can always convert 20 bytes to account::Id"),
            )
            .context("validator not found")?
            .pub_key;
        let verification_key = VerificationKey::try_from(pubkey.to_bytes().as_slice())
            .context("failed to create verification key")?;

        let vote_extension = CanonicalVoteExtension {
            extension: vote.vote_extension.to_vec(),
            height: (height - 1) as i64,
            round: extended_commit_info.round.value() as i64,
            chain_id: chain_id.to_string(),
        };

        // TODO: double check that it's length-delimited
        let message = vote_extension.encode_length_delimited_to_vec();
        let signature = Signature::try_from(
            vote.extension_signature
                .as_ref()
                .expect("extension signature is some, as it was checked above")
                .as_bytes(),
        )
        .context("failed to create signature")?;
        verification_key
            .verify(&signature, &message)
            .context("failed to verify signature for vote extension")?;
    }

    // this shouldn't happen, but good to check anyways
    if total_voting_power == 0 {
        anyhow::bail!("total voting power is zero");
    }

    let required_voting_power = total_voting_power
        .checked_mul(2)
        .context("failed to multiply total voting power by 2")?
        .checked_div(3)
        .context("failed to divide total voting power by 3")?
        .checked_sub(1)
        .context("failed to subtract 1 from total voting power")?;
    ensure!(
        submitted_voting_power >= required_voting_power,
        "submitted voting power is less than required voting power",
    );

    debug!(
        submitted_voting_power,
        total_voting_power, "validated extended commit info"
    );
    Ok(())
}

fn validate_extended_commit_against_last_commit(
    last_commit: &CommitInfo,
    extended_commit_info: &ExtendedCommitInfo,
) -> anyhow::Result<()> {
    ensure!(
        last_commit.round == extended_commit_info.round,
        "last commit round does not match extended commit round"
    );

    ensure!(
        last_commit.votes.len() == extended_commit_info.votes.len(),
        "last commit votes length does not match extended commit votes length"
    );

    ensure!(
        is_sorted::IsSorted::is_sorted_by(&mut extended_commit_info.votes.iter(), |a, b| {
            if a.validator.power == b.validator.power {
                // addresses sorted in ascending order, if the powers are the same
                a.validator.address.partial_cmp(&b.validator.address)
            } else {
                // powers sorted in descending order
                a.validator
                    .power
                    .partial_cmp(&b.validator.power)
                    .map(|v| v.reverse())
            }
        }),
        "extended commit votes are not sorted by voting power",
    );

    for (i, vote) in extended_commit_info.votes.iter().enumerate() {
        let last_commit_vote = &last_commit.votes[i];
        ensure!(
            last_commit_vote.validator.address == vote.validator.address,
            "last commit vote address does not match extended commit vote address"
        );
        ensure!(
            last_commit_vote.validator.power == vote.validator.power,
            "last commit vote power does not match extended commit vote power"
        );

        // vote is absent; no need to check for the block id flag matching the last commit
        if vote.sig_info == Flag(tendermint::block::BlockIdFlag::Absent)
            && vote.vote_extension.len() == 0
            && vote.extension_signature.is_none()
        {
            continue;
        }

        ensure!(
            vote.sig_info == last_commit_vote.sig_info,
            "last commit vote sig info does not match extended commit vote sig info"
        );
    }

    Ok(())
}

pub(crate) async fn apply_prices_from_vote_extensions<S: StateWriteExt>(
    state: &mut S,
    extended_commit_info: ExtendedCommitInfo,
    timestamp: Timestamp,
    height: u64,
) -> anyhow::Result<()> {
    let votes = extended_commit_info
        .votes
        .iter()
        .map(|vote| {
            let raw = RawOracleVoteExtension::decode(vote.vote_extension.clone())
                .context("failed to decode oracle vote extension")?;
            Ok(OracleVoteExtension::from_raw(raw))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let prices = aggregate_oracle_votes(state, votes)
        .await
        .context("failed to aggregate oracle votes")?;

    // TODO: if a currency pair exists in the state, but isn't in the prices,
    // is it considered "removed"?
    for (currency_pair, price) in prices {
        let price = QuotePrice {
            price,
            block_timestamp: pbjson_types::Timestamp {
                seconds: timestamp.seconds,
                nanos: timestamp.nanos,
            },
            block_height: height,
        };

        state
            .put_price_for_currency_pair(&currency_pair, price)
            .context("failed to put price")?;
    }

    Ok(())
}

async fn aggregate_oracle_votes<S: StateReadExt>(
    state: &S,
    votes: Vec<OracleVoteExtension>,
) -> anyhow::Result<HashMap<CurrencyPair, u128>> {
    // validators are not weighted right now, so we just take the median price for each currency
    // pair
    //
    // skip uses a stake-weighted median: https://github.com/skip-mev/slinky/blob/19a916122110cfd0e98d93978107d7ada1586918/pkg/math/voteweighted/voteweighted.go#L59
    // we can implement this later, when we have stake weighting.
    let mut currency_pair_to_price_list = HashMap::new();
    for vote in votes {
        for (id, price_bytes) in vote.prices {
            if price_bytes.len() > MAXIMUM_PRICE_BYTE_LEN {
                continue;
            }

            let Some(currency_pair) = DefaultCurrencyPairStrategy::from_id(state, id)
                .await
                .context("failed to get currency pair from id")?
            else {
                continue;
            };

            let price =
                DefaultCurrencyPairStrategy::get_decoded_price(state, &currency_pair, &price_bytes)
                    .await
                    .context("failed to get decoded price")?;
            currency_pair_to_price_list
                .entry(currency_pair)
                .and_modify(|prices: &mut Vec<u128>| prices.push(price))
                .or_insert(vec![price]);
        }
    }

    let mut prices = HashMap::new();
    for (currency_pair, mut price_list) in currency_pair_to_price_list {
        let median_price = if price_list.is_empty() {
            // price list should not ever be empty,
            // as it was only inserted if it had a price
            0
        } else {
            price_list.sort_unstable();
            let mid = price_list.len() / 2;
            if price_list.len() % 2 == 0 {
                (price_list[mid - 1] + price_list[mid]) / 2
            } else {
                price_list[mid]
            }
        };
        prices.insert(currency_pair, median_price);
    }

    Ok(prices)
}
