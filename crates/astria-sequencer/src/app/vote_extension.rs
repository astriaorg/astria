use std::collections::HashMap;

use anyhow::{
    bail,
    ensure,
    Context as _,
};
use astria_core::{
    crypto::Signature,
    generated::astria_vendored::slinky::{
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
use indexmap::IndexMap;
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
use tracing::{
    debug,
    trace,
};

use crate::{
    address::StateReadExt as _,
    authority::StateReadExt as _,
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
}

impl Handler {
    pub(crate) fn new(oracle_client: Option<OracleClient<Channel>>) -> Self {
        Self {
            oracle_client,
        }
    }

    pub(crate) async fn extend_vote<S: StateReadExt>(
        &mut self,
        state: &S,
    ) -> anyhow::Result<abci::response::ExtendVote> {
        let Some(oracle_client) = self.oracle_client.as_mut() else {
            // we allow validators to *not* use the oracle sidecar currently,
            // so this will get converted to an empty vote extension when bubbled up.
            //
            // however, if >1/3 of validators are not using the oracle, the prices will not update.
            bail!("oracle client not set")
        };

        // if we fail to get prices within the timeout duration, we will return an empty vote
        // extension to ensure liveness.
        let prices = match oracle_client.prices(QueryPricesRequest {}).await {
            Ok(prices) => prices.into_inner(),
            Err(e) => {
                bail!("failed to get prices from oracle sidecar: {e}",);
            }
        };

        tracing::debug!(
            prices_count = prices.prices.len(),
            "got prices from oracle sidecar; transforming prices"
        );

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
    ) -> abci::response::VerifyVoteExtension {
        if vote.vote_extension.is_empty() {
            return abci::response::VerifyVoteExtension::Accept;
        }

        match verify_vote_extension(state, vote.vote_extension, false).await {
            Ok(()) => abci::response::VerifyVoteExtension::Accept,
            Err(e) => {
                tracing::error!(error = %e, "failed to verify vote extension");
                abci::response::VerifyVoteExtension::Reject
            }
        }
    }
}

// see https://github.com/skip-mev/slinky/blob/5b07f91d6c0110e617efda3f298f147a31da0f25/abci/ve/utils.go#L24
async fn verify_vote_extension<S: StateReadExt>(
    state: &S,
    oracle_vote_extension_bytes: bytes::Bytes,
    is_proposal_phase: bool,
) -> anyhow::Result<()> {
    let oracle_vote_extension = RawOracleVoteExtension::decode(oracle_vote_extension_bytes)
        .context("failed to decode oracle vote extension")?;
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
    let mut strategy_prices = IndexMap::new();
    for (currency_pair_id, price_string) in prices.prices {
        let currency_pair = currency_pair_id
            .parse()
            .context("failed to parse currency pair")?;

        // prices are encoded as just a decimal string in the sidecar response
        let price: u128 = price_string
            .parse()
            .context("failed to parse price string")?;

        let Ok(id) = DefaultCurrencyPairStrategy::id(state, &currency_pair).await else {
            trace!(
                currency_pair = currency_pair.to_string(),
                "currency pair not found in state; skipping"
            );
            continue;
        };
        let encoded_price = DefaultCurrencyPairStrategy::get_encoded_price(state, price);

        debug!(
            currency_pair = currency_pair.to_string(),
            id, price, "transformed price for inclusion in vote extension"
        );
        strategy_prices.insert(id, encoded_price.into());
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

        for vote in &mut extended_commit_info.votes {
            if let Err(e) = verify_vote_extension(state, vote.vote_extension.clone(), true).await {
                let address = state
                    .try_base_prefixed(vote.validator.address.as_slice())
                    .await
                    .context("failed to construct validator address with base prefix")?;
                debug!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    validator = address.to_string(),
                    "failed to verify vote extension; pruning from proposal"
                );
                vote.sig_info = Flag(tendermint::block::BlockIdFlag::Absent);
                vote.extension_signature = None;
                vote.vote_extension.clear();
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
        let address = state
            .try_base_prefixed(vote.validator.address.as_slice())
            .await
            .context("failed to construct validator address with base prefix")?;

        total_voting_power = total_voting_power.saturating_add(vote.validator.power.value());

        if vote.sig_info == Flag(tendermint::block::BlockIdFlag::Commit)
            && vote.extension_signature.is_none()
        {
            anyhow::bail!("vote extension signature is missing for validator {address}",);
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit)
            && !vote.vote_extension.is_empty()
        {
            anyhow::bail!("non-commit vote extension present for validator {address}",);
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit)
            && vote.extension_signature.is_some()
        {
            anyhow::bail!("non-commit extension signature present for validator {address}",);
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit) {
            continue;
        }

        submitted_voting_power =
            submitted_voting_power.saturating_add(vote.validator.power.value());

        let verification_key = &validator_set
            .get(vote.validator.address)
            .context("validator not found")?
            .verification_key;

        let vote_extension = CanonicalVoteExtension {
            extension: vote.vote_extension.to_vec(),
            height: i64::try_from(height.checked_sub(1).expect(
                "can subtract 1 from height as this function is only called for block height >1",
            ))
            .expect("block height must fit in an i64"),
            round: i64::from(extended_commit_info.round.value()),
            chain_id: chain_id.to_string(),
        };

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
        .checked_add(1)
        .context("failed to add 1 from total voting power")?;
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
                    .map(std::cmp::Ordering::reverse)
            }
        }),
        "extended commit votes are not sorted by voting power",
    );

    for (last_commit_vote, extended_commit_info_vote) in last_commit
        .votes
        .iter()
        .zip(extended_commit_info.votes.iter())
    {
        ensure!(
            last_commit_vote.validator.address == extended_commit_info_vote.validator.address,
            "last commit vote address does not match extended commit vote address"
        );
        ensure!(
            last_commit_vote.validator.power == extended_commit_info_vote.validator.power,
            "last commit vote power does not match extended commit vote power"
        );

        // vote is absent; no need to check for the block id flag matching the last commit
        if extended_commit_info_vote.sig_info == Flag(tendermint::block::BlockIdFlag::Absent)
            && extended_commit_info_vote.vote_extension.is_empty()
            && extended_commit_info_vote.extension_signature.is_none()
        {
            continue;
        }

        ensure!(
            extended_commit_info_vote.sig_info == last_commit_vote.sig_info,
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

    for (currency_pair, price) in prices {
        let price = QuotePrice {
            price,
            block_timestamp: astria_core::Timestamp {
                seconds: timestamp.seconds,
                nanos: timestamp.nanos,
            },
            block_height: height,
        };

        tracing::debug!(
            currency_pair = currency_pair.to_string(),
            price = price.price,
            "applied price from vote extension"
        );

        state
            .put_price_for_currency_pair(&currency_pair, price)
            .await
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

            let price = DefaultCurrencyPairStrategy::get_decoded_price(state, &price_bytes)
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
                let num_to_skip = mid
                    .checked_sub(1)
                    .expect("must subtract as the length of the price list is >0");
                price_list.iter().skip(num_to_skip).take(2).sum::<u128>() / 2
            } else {
                price_list
                    .get(mid)
                    .copied()
                    .expect("must have element as mid < len")
            }
        };
        prices.insert(currency_pair, median_price);
    }

    Ok(prices)
}

#[cfg(test)]
mod test {
    use astria_core::{
        crypto::SigningKey,
        protocol::transaction::v1alpha1::action::ValidatorUpdate,
    };
    use cnidarium::StateDelta;
    use tendermint::abci::types::{
        ExtendedVoteInfo,
        Validator,
    };
    use tendermint_proto::types::CanonicalVoteExtension;

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        authority::{
            StateWriteExt as _,
            ValidatorSet,
        },
        state_ext::StateWriteExt as _,
    };

    #[tokio::test]
    async fn verify_vote_extension_proposal_phase_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        verify_vote_extension(&snapshot, vec![].into(), true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn verify_vote_extension_not_proposal_phase_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        verify_vote_extension(&snapshot, vec![].into(), true)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn validate_vote_extensions_insufficient_voting_power() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(&snapshot);
        state.put_chain_id_and_revision_number("test-0".try_into().unwrap());
        let validator_set = ValidatorSet::new_from_updates(vec![
            ValidatorUpdate {
                power: 1u16.into(),
                verification_key: SigningKey::from([0; 32]).verification_key(),
            },
            ValidatorUpdate {
                power: 2u16.into(),
                verification_key: SigningKey::from([1; 32]).verification_key(),
            },
        ]);
        state.put_validator_set(validator_set).unwrap();
        state.put_base_prefix("astria");

        let extended_commit_info = ExtendedCommitInfo {
            round: 1u16.into(),
            votes: vec![ExtendedVoteInfo {
                validator: Validator {
                    address: SigningKey::from([0; 32]).verification_key().address_bytes(),
                    power: 1u16.into(),
                },
                sig_info: Flag(tendermint::block::BlockIdFlag::Nil),
                extension_signature: None,
                vote_extension: vec![].into(),
            }],
        };
        assert!(
            validate_vote_extensions(&state, 1, &extended_commit_info)
                .await
                .unwrap_err()
                .to_string()
                .contains("submitted voting power is less than required voting power")
        );
    }

    #[tokio::test]
    async fn validate_vote_extensions_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(&snapshot);

        let chain_id: tendermint::chain::Id = "test-0".try_into().unwrap();
        state.put_chain_id_and_revision_number(chain_id.clone());
        let validator_set = ValidatorSet::new_from_updates(vec![
            ValidatorUpdate {
                power: 5u16.into(),
                verification_key: SigningKey::from([0; 32]).verification_key(),
            },
            ValidatorUpdate {
                power: 2u16.into(),
                verification_key: SigningKey::from([1; 32]).verification_key(),
            },
        ]);
        state.put_validator_set(validator_set).unwrap();
        state.put_base_prefix("astria");

        let round = 1u16;
        let vote_extension_height = 1u64;
        let vote_extension_message = b"noot".to_vec();
        let vote_extension = CanonicalVoteExtension {
            extension: vote_extension_message.clone(),
            height: vote_extension_height.try_into().unwrap(),
            round: i64::from(round),
            chain_id: chain_id.to_string(),
        };

        let message = vote_extension.encode_length_delimited_to_vec();
        let signature = SigningKey::from([0; 32]).sign(&message);

        let extended_commit_info = ExtendedCommitInfo {
            round: round.into(),
            votes: vec![ExtendedVoteInfo {
                validator: Validator {
                    address: SigningKey::from([0; 32]).verification_key().address_bytes(),
                    power: 1u16.into(),
                },
                sig_info: Flag(tendermint::block::BlockIdFlag::Commit),
                extension_signature: Some(signature.to_bytes().to_vec().try_into().unwrap()),
                vote_extension: vote_extension_message.into(),
            }],
        };
        validate_vote_extensions(&state, vote_extension_height + 1, &extended_commit_info)
            .await
            .unwrap();
    }
}
