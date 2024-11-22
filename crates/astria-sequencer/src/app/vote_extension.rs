use std::collections::HashSet;

use astria_core::{
    connect::{
        abci::v2::OracleVoteExtension,
        oracle::v2::QuotePrice,
        service::v2::QueryPricesResponse,
        types::v2::{
            CurrencyPair,
            CurrencyPairId,
            Price,
        },
    },
    crypto::Signature,
    generated::connect::{
        abci::v2::OracleVoteExtension as RawOracleVoteExtension,
        service::v2::{
            oracle_client::OracleClient,
            QueryPricesRequest,
        },
    },
    protocol::connect::v1::ExtendedCommitInfoWithCurrencyPairMapping,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    ContextCompat as _,
    Result,
    WrapErr as _,
};
use futures::{
    StreamExt as _,
    TryStreamExt,
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
    info,
    instrument,
    warn,
};

use crate::{
    address::StateReadExt as _,
    app::state_ext::StateReadExt,
    authority::StateReadExt as _,
    connect::oracle::{
        currency_pair_strategy::DefaultCurrencyPairStrategy,
        state_ext::StateWriteExt,
    },
};

// https://github.com/skip-mev/connect/blob/793b2e874d6e720bd288e82e782502e41cf06f8c/abci/types/constants.go#L6
const MAXIMUM_PRICE_BYTE_LEN: usize = 33;

pub(crate) struct Handler {
    // gRPC client for the connect oracle sidecar.
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
    ) -> Result<abci::response::ExtendVote> {
        let Some(oracle_client) = self.oracle_client.as_mut() else {
            // we allow validators to *not* use the oracle sidecar currently,
            // so this will get converted to an empty vote extension when bubbled up.
            //
            // however, if >1/3 of validators are not using the oracle, the prices will not update.
            bail!("oracle client not set")
        };

        // if we fail to get prices within the timeout duration, we will return an empty vote
        // extension to ensure liveness.
        let rsp = match oracle_client.prices(QueryPricesRequest {}).await {
            Ok(rsp) => rsp.into_inner(),
            Err(e) => {
                bail!("failed to get prices from oracle sidecar: {e:#}",);
            }
        };

        let query_prices_response =
            astria_core::connect::service::v2::QueryPricesResponse::try_from_raw(rsp)
                .wrap_err("failed to validate prices server response")?;
        let oracle_vote_extension = transform_oracle_service_prices(state, query_prices_response)
            .await
            .wrap_err("failed to transform oracle service prices")?;

        Ok(abci::response::ExtendVote {
            vote_extension: oracle_vote_extension.into_raw().encode_to_vec().into(),
        })
    }

    pub(crate) async fn verify_vote_extension<S: StateReadExt>(
        &self,
        state: &S,
        vote: abci::request::VerifyVoteExtension,
    ) -> Result<abci::response::VerifyVoteExtension> {
        if vote.vote_extension.is_empty() {
            return Ok(abci::response::VerifyVoteExtension::Accept);
        }

        let max_num_currency_pairs =
            DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state, false)
                .await
                .wrap_err("failed to get max number of currency pairs")?;

        let response = match verify_vote_extension(vote.vote_extension, max_num_currency_pairs) {
            Ok(_) => abci::response::VerifyVoteExtension::Accept,
            Err(e) => {
                tracing::warn!(error = %e, "failed to verify vote extension");
                abci::response::VerifyVoteExtension::Reject
            }
        };
        Ok(response)
    }
}

// see https://github.com/skip-mev/connect/blob/5b07f91d6c0110e617efda3f298f147a31da0f25/abci/ve/utils.go#L24
fn verify_vote_extension(
    oracle_vote_extension_bytes: bytes::Bytes,
    max_num_currency_pairs: u64,
) -> Result<HashSet<u64>> {
    let oracle_vote_extension = RawOracleVoteExtension::decode(oracle_vote_extension_bytes)
        .wrap_err("failed to decode oracle vote extension")?;

    ensure!(
        u64::try_from(oracle_vote_extension.prices.len()).ok() <= Some(max_num_currency_pairs),
        "number of oracle vote extension prices exceeds max expected number of currency pairs"
    );

    let mut ids = HashSet::with_capacity(oracle_vote_extension.prices.len());
    for (id, price) in oracle_vote_extension.prices {
        ensure!(
            price.len() <= MAXIMUM_PRICE_BYTE_LEN,
            "encoded price length exceeded {MAXIMUM_PRICE_BYTE_LEN}"
        );
        ids.insert(id);
    }

    Ok(ids)
}

// see https://github.com/skip-mev/connect/blob/158cde8a4b774ac4eec5c6d1a2c16de6a8c6abb5/abci/ve/vote_extension.go#L290
#[instrument(skip_all)]
async fn transform_oracle_service_prices<S: StateReadExt>(
    state: &S,
    rsp: QueryPricesResponse,
) -> Result<OracleVoteExtension> {
    use astria_core::connect::types::v2::CurrencyPairId;

    let futures = futures::stream::FuturesUnordered::new();
    for (currency_pair, price) in rsp.prices {
        futures.push(async move {
            (
                DefaultCurrencyPairStrategy::id(state, &currency_pair).await,
                currency_pair,
                price,
            )
        });
    }

    let result: Vec<(Result<Option<CurrencyPairId>>, CurrencyPair, Price)> =
        futures.collect().await;
    let strategy_prices = result.into_iter().filter_map(|(get_id_result, currency_pair, price)| {
        let id = match get_id_result {
            Ok(Some(id)) => id,
            Ok(None) => {
                debug!(%currency_pair, "currency pair ID not found in state; skipping");
                return None;
            }
            Err(err) => {
                // FIXME: this event can be removed once all instrumented functions
                // can generate an error event.
                warn!(%currency_pair, "failed to fetch ID for currency pair; cancelling transformation");
                return Some(Err(err).wrap_err("failed to fetch currency pair ID"));
            }
        };
        Some(Ok((id, price)))
    }).collect::<Result<IndexMap<CurrencyPairId, Price>>>()?;

    Ok(OracleVoteExtension {
        prices: strategy_prices,
    })
}

pub(crate) struct ProposalHandler;

impl ProposalHandler {
    // called during prepare_proposal; prunes and validates the local extended commit info
    // received during the previous block's voting period.
    //
    // the returned extended commit info will be proposed this block.
    pub(crate) async fn prepare_proposal<S: StateReadExt>(
        state: &S,
        height: u64,
        mut extended_commit_info: ExtendedCommitInfo,
    ) -> Result<ExtendedCommitInfoWithCurrencyPairMapping> {
        if height == 1 {
            // we're proposing block 1, so nothing to validate
            info!(
                "skipping vote extension proposal for block 1, as there were no previous vote \
                 extensions"
            );
            return Ok(ExtendedCommitInfoWithCurrencyPairMapping::new(
                extended_commit_info,
                IndexMap::new(),
            ));
        }

        let max_num_currency_pairs =
            DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state, true)
                .await
                .wrap_err("failed to get max number of currency pairs")?;

        let mut all_ids = HashSet::new();
        for vote in &mut extended_commit_info.votes {
            let ids =
                match verify_vote_extension(vote.vote_extension.clone(), max_num_currency_pairs) {
                    Ok(ids) => ids,
                    Err(e) => {
                        let address = state
                            .try_base_prefixed(vote.validator.address.as_slice())
                            .await
                            .wrap_err("failed to construct validator address with base prefix")?;
                        debug!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            validator = address.to_string(),
                            "failed to verify vote extension; pruning from proposal"
                        );
                        vote.sig_info = Flag(tendermint::block::BlockIdFlag::Absent);
                        vote.extension_signature = None;
                        vote.vote_extension.clear();
                        continue;
                    }
                };
            all_ids.extend(ids);
        }

        validate_vote_extensions(state, height, &extended_commit_info)
            .await
            .wrap_err("failed to validate vote extensions in prepare_proposal")?;

        let futures = futures::stream::FuturesUnordered::new();
        for id in all_ids {
            let id = CurrencyPairId::new(id);
            futures
                .push(async move { (DefaultCurrencyPairStrategy::from_id(state, id).await, id) });
        }
        let result = futures
            .collect::<Vec<(Result<Option<CurrencyPair>>, CurrencyPairId)>>()
            .await;
        let id_to_currency_pair = result.into_iter().filter_map(|(result, id)| {
            let currency_pair = match result {
                Ok(Some(currency_pair)) => currency_pair,
                Ok(None) => {
                    debug!(%id, "currency pair not found in state; skipping");
                    return None;
                }
                Err(e) => {
                    // FIXME: this event can be removed once all instrumented functions
                    // can generate an error event.
                    warn!(%id, error = AsRef::<dyn std::error::Error>::as_ref(&e), "failed to fetch currency pair for ID; skipping");
                    return None;
                }
            };
            Some((id, currency_pair))
        }).collect::<IndexMap<CurrencyPairId, CurrencyPair>>();

        let tx = ExtendedCommitInfoWithCurrencyPairMapping::new(
            extended_commit_info,
            id_to_currency_pair,
        );

        Ok(tx)
    }

    // called during process_proposal; validates the proposed extended commit info.
    pub(crate) async fn validate_proposal<S: StateReadExt>(
        state: &S,
        height: u64,
        last_commit: &CommitInfo,
        extended_commit_info: &ExtendedCommitInfoWithCurrencyPairMapping,
    ) -> Result<()> {
        let ExtendedCommitInfoWithCurrencyPairMapping {
            extended_commit_info,
            id_to_currency_pair,
        } = extended_commit_info;

        if height == 1 {
            // we're processing block 1, so nothing to validate (no last commit yet)
            info!(
                "skipping vote extension validation for block 1, as there were no previous vote \
                 extensions"
            );
            return Ok(());
        }

        validate_id_to_currency_pair_mapping(state, id_to_currency_pair)
            .await
            .wrap_err("failed to validate id_to_currency_pair mapping")?;

        if extended_commit_info.votes.is_empty() {
            ensure!(
                last_commit.round == extended_commit_info.round,
                "last commit round does not match extended commit round"
            );

            // it's okay for the extended commit info to be empty,
            // as it's possible the proposer did not receive valid vote extensions from >2/3 staking
            // power.
            return Ok(());
        }

        // inside process_proposal, we must validate the vote extensions proposed against the last
        // commit proposed
        validate_extended_commit_against_last_commit(last_commit, extended_commit_info)?;

        validate_vote_extensions(state, height, extended_commit_info)
            .await
            .wrap_err("failed to validate vote extensions in validate_extended_commit_info")?;

        let max_num_currency_pairs =
            DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state, true)
                .await
                .wrap_err("failed to get max number of currency pairs")?;

        for vote in &extended_commit_info.votes {
            verify_vote_extension(vote.vote_extension.clone(), max_num_currency_pairs)
                .wrap_err("failed to verify vote extension in validate_proposal")?;
        }

        Ok(())
    }
}

async fn validate_id_to_currency_pair_mapping<S: StateReadExt>(
    state: &S,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPair>,
) -> Result<()> {
    let mut futures = futures::stream::FuturesUnordered::new();
    for (id, currency_pair) in id_to_currency_pair {
        futures.push(async move {
            let expected_currency_pair = DefaultCurrencyPairStrategy::from_id(state, *id)
                .await
                .wrap_err("failed to get currency pair for id")?
                .ok_or(eyre!("currency pair should exist in state"))?;
            ensure!(
                currency_pair == &expected_currency_pair,
                format!(
                    "currency pair {} was not expected {} given id {}",
                    currency_pair, expected_currency_pair, id
                )
            );
            Ok(())
        });
    }
    while futures.try_next().await?.is_some() {}

    Ok(())
}

// see https://github.com/skip-mev/connect/blob/5b07f91d6c0110e617efda3f298f147a31da0f25/abci/ve/utils.go#L111
async fn validate_vote_extensions<S: StateReadExt>(
    state: &S,
    height: u64,
    extended_commit_info: &ExtendedCommitInfo,
) -> Result<()> {
    use tendermint_proto::v0_38::types::CanonicalVoteExtension;

    let chain_id = state
        .get_chain_id()
        .await
        .wrap_err("failed to get chain id")?;

    // total validator voting power
    let mut total_voting_power: u64 = 0;
    // the total voting power of all validators which submitted vote extensions
    let mut submitted_voting_power: u64 = 0;

    let validator_set = state
        .get_validator_set()
        .await
        .wrap_err("failed to get validator set")?;

    for vote in &extended_commit_info.votes {
        let address = state
            .try_base_prefixed(vote.validator.address.as_slice())
            .await
            .wrap_err("failed to construct validator address with base prefix")?;

        total_voting_power = total_voting_power.saturating_add(vote.validator.power.value());

        if vote.sig_info == Flag(tendermint::block::BlockIdFlag::Commit) {
            ensure!(
                vote.extension_signature.is_some(),
                "vote extension signature is missing for validator {address}",
            );
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit) {
            ensure!(
                vote.vote_extension.is_empty(),
                "non-commit vote extension present for validator {address}"
            );
            ensure!(
                vote.extension_signature.is_none(),
                "non-commit extension signature present for validator {address}",
            );
        }

        if vote.sig_info != Flag(tendermint::block::BlockIdFlag::Commit) {
            continue;
        }

        submitted_voting_power =
            submitted_voting_power.saturating_add(vote.validator.power.value());

        let verification_key = &validator_set
            .get(&vote.validator.address)
            .wrap_err("validator not found")?
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
        .wrap_err("failed to create signature")?;
        verification_key
            .verify(&signature, &message)
            .wrap_err("failed to verify signature for vote extension")?;
    }

    // this shouldn't happen, but good to check anyways
    if total_voting_power == 0 {
        bail!("total voting power is zero");
    }

    let required_voting_power = total_voting_power
        .checked_mul(2)
        .wrap_err("failed to multiply total voting power by 2")?
        .checked_div(3)
        .wrap_err("failed to divide total voting power by 3")?
        .checked_add(1)
        .wrap_err("failed to add 1 from total voting power")?;
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
) -> Result<()> {
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
    extended_commit_info: ExtendedCommitInfoWithCurrencyPairMapping,
    timestamp: Timestamp,
    height: u64,
) -> Result<()> {
    let ExtendedCommitInfoWithCurrencyPairMapping {
        extended_commit_info,
        id_to_currency_pair,
    } = extended_commit_info;

    let prices = astria_core::connect::utils::calculate_prices_from_vote_extensions(
        extended_commit_info,
        &id_to_currency_pair,
    )
    .await
    .wrap_err("failed to calculate prices from vote extensions")?;
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

#[cfg(test)]
mod test {
    use astria_core::{
        crypto::SigningKey,
        protocol::transaction::v1::action::ValidatorUpdate,
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
        app::StateWriteExt as _,
        authority::{
            StateWriteExt as _,
            ValidatorSet,
        },
    };

    #[test]
    fn verify_vote_extension_empty_ok() {
        verify_vote_extension(vec![].into(), 100).unwrap();
    }

    #[tokio::test]
    async fn validate_vote_extensions_insufficient_voting_power() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(&snapshot);
        state
            .put_chain_id_and_revision_number("test-0".try_into().unwrap())
            .unwrap();
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
        state.put_base_prefix("astria".to_string()).unwrap();

        let extended_commit_info = ExtendedCommitInfo {
            round: 1u16.into(),
            votes: vec![ExtendedVoteInfo {
                validator: Validator {
                    address: *SigningKey::from([0; 32]).verification_key().address_bytes(),
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
        state
            .put_chain_id_and_revision_number(chain_id.clone())
            .unwrap();
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
        state.put_base_prefix("astria".to_string()).unwrap();

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
                    address: *SigningKey::from([0; 32]).verification_key().address_bytes(),
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
