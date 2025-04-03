use std::collections::HashSet;

use astria_core::{
    crypto::Signature,
    generated::price_feed::{
        abci::v2::OracleVoteExtension as RawOracleVoteExtension,
        service::v2::{
            oracle_client::OracleClient,
            QueryPricesRequest,
        },
    },
    oracles::price_feed::{
        abci::v2::OracleVoteExtension,
        oracle::v2::QuotePrice,
        service::v2::QueryPricesResponse,
        types::v2::{
            CurrencyPairId,
            Price,
        },
    },
    protocol::price_feed::v1::{
        CurrencyPairInfo,
        ExtendedCommitInfoWithCurrencyPairMapping,
    },
};
use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use futures::{
    stream::FuturesUnordered,
    StreamExt as _,
    TryStreamExt,
};
use indexmap::IndexMap;
use itertools::Itertools as _;
use prost::Message as _;
use telemetry::display::base64;
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
    oracles::price_feed::oracle::{
        currency_pair_strategy::DefaultCurrencyPairStrategy,
        state_ext::StateWriteExt,
    },
};

// https://github.com/skip-mev/connect/blob/793b2e874d6e720bd288e82e782502e41cf06f8c/abci/types/constants.go#L6
const MAXIMUM_PRICE_BYTE_LEN: usize = 33;

pub(crate) struct Handler {
    // gRPC client for the price feed oracle sidecar.
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

        let query_prices_response = QueryPricesResponse::try_from_raw(rsp)
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

        let max_num_currency_pairs = DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state)
            .await
            .wrap_err("failed to get max number of currency pairs")?;

        let response = match verify_vote_extension(vote.vote_extension, max_num_currency_pairs) {
            Ok(_) => abci::response::VerifyVoteExtension::Accept,
            Err(e) => {
                warn!(error = %e, "failed to verify vote extension");
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

    let num_prices = u64::try_from(oracle_vote_extension.prices.len()).wrap_err_with(|| {
        format!(
            "expected no more than {} prices, got {} prices",
            u64::MAX,
            oracle_vote_extension.prices.len()
        )
    })?;
    ensure!(
        num_prices <= max_num_currency_pairs,
        "number of oracle vote extension prices exceeds max expected number of currency pairs"
    );

    let mut ids = HashSet::with_capacity(oracle_vote_extension.prices.len());
    for (id, price) in oracle_vote_extension.prices {
        ensure!(
            price.len() <= MAXIMUM_PRICE_BYTE_LEN,
            "encoded price length exceeded {MAXIMUM_PRICE_BYTE_LEN} bytes"
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
    use astria_core::oracles::price_feed::types::v2::CurrencyPairId;

    let strategy_prices = rsp.prices.into_iter().map(|(currency_pair, price)| async move {
        DefaultCurrencyPairStrategy::id(state, &currency_pair).await
            .wrap_err_with(|| {
                warn!(%currency_pair, "failed to fetch ID for currency pair; cancelling transformation");
                format!("error fetching currency pair {currency_pair}")
            })
            .map(|maybe_id| (maybe_id, currency_pair, price))
    }).collect::<FuturesUnordered<_>>()
        .try_filter_map(|(maybe_id, currency_pair, price)| async move {
            let Some(id) = maybe_id else {
                debug!(%currency_pair, "currency pair ID not found in state; skipping");
                return Ok(None);
            };
            Ok(Some((id, price)))
        })
        .try_collect::<IndexMap<CurrencyPairId, Price>>().await?;

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

        let max_num_currency_pairs = DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state)
            .await
            .wrap_err("failed to get max number of currency pairs")?;

        let mut all_currency_pair_ids = HashSet::new();
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
            all_currency_pair_ids.extend(ids);
        }

        validate_vote_extensions(state, height, &extended_commit_info)
            .await
            .wrap_err("failed to validate vote extensions in prepare_proposal")?;

        let id_to_currency_pair = get_id_to_currency_pair(&state, all_currency_pair_ids)
            .await
            .wrap_err("failed to get id to currency pair mapping")?;
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

        let max_num_currency_pairs = DefaultCurrencyPairStrategy::get_max_num_currency_pairs(state)
            .await
            .wrap_err("failed to get max number of currency pairs")?;

        let mut all_currency_pair_ids = HashSet::new();
        for vote in &extended_commit_info.votes {
            let ids = verify_vote_extension(vote.vote_extension.clone(), max_num_currency_pairs)
                .wrap_err("failed to verify vote extension in validate_proposal")?;
            all_currency_pair_ids.extend(ids);
        }

        validate_id_to_currency_pair_mapping(state, all_currency_pair_ids, id_to_currency_pair)
            .await
    }
}

async fn get_id_to_currency_pair<S: StateReadExt>(
    state: &S,
    all_ids: HashSet<u64>,
) -> Result<IndexMap<CurrencyPairId, CurrencyPairInfo>> {
    use crate::oracles::price_feed::market_map::state_ext::StateReadExt as _;
    let market_map = state
        .get_market_map()
        .await
        .wrap_err("failed to get market map")?
        .ok_or(eyre!("market map was not set"))?;

    let num_pairs = all_ids.len();
    let mut id_to_currency_pair_stream = all_ids
        .into_iter()
        .map(|id| async move {
            let pair_id = CurrencyPairId::new(id);
            let res = DefaultCurrencyPairStrategy::from_id(state, pair_id)
                .await
                .wrap_err_with(|| format!("failed to get currency pair from id {id}"));
            (pair_id, res)
        })
        .collect::<FuturesUnordered<_>>();

    let mut id_to_currency_pair = IndexMap::with_capacity(num_pairs);
    while let Some((id, result)) = id_to_currency_pair_stream.next().await {
        match result {
            Ok(Some(currency_pair)) => {
                let currency_pair_str = currency_pair.to_string();
                let info = CurrencyPairInfo {
                    currency_pair,
                    decimals: market_map
                        .markets
                        .get(&currency_pair_str)
                        .ok_or(eyre!("currency pair did not exist in market map"))?
                        .ticker
                        .decimals,
                };
                let _ = id_to_currency_pair.insert(id, info);
            }
            Ok(None) => {
                debug!(%id, "currency pair not found in state; skipping");
            }
            Err(e) => {
                // FIXME: this event can be removed once all instrumented functions
                // can generate an error event.
                warn!(
                    %id, error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed to fetch currency pair for ID; skipping"
                );
            }
        }
    }
    Ok(id_to_currency_pair)
}

async fn validate_id_to_currency_pair_mapping<S: StateReadExt>(
    state: &S,
    all_ids: HashSet<u64>,
    id_to_currency_pair: &IndexMap<CurrencyPairId, CurrencyPairInfo>,
) -> Result<()> {
    let mut expected_mapping = get_id_to_currency_pair(state, all_ids)
        .await
        .wrap_err("failed to get id to currency pair mapping")?;
    if expected_mapping == *id_to_currency_pair {
        return Ok(());
    }

    let mut error_msgs = vec![];
    let mut actual_mapping = id_to_currency_pair.clone();
    for (pair_id, expected_pair) in expected_mapping.drain(..) {
        if let Some(actual_pair) = actual_mapping.swap_remove(&pair_id) {
            if expected_pair != actual_pair {
                error_msgs.push(format!(
                    "mismatch (expected `{expected_pair}` but got `{actual_pair}` for id \
                     {pair_id})"
                ));
            }
        } else {
            error_msgs.push(format!(
                "missing (expected `{expected_pair}` for id {pair_id})"
            ));
        }
    }
    for (pair_id, extra_pair) in actual_mapping {
        error_msgs.push(format!("unexpected (got `{extra_pair}` for id {pair_id})"));
    }
    Err(eyre!(
        "failed to validate currency pair mapping: [{}]",
        error_msgs.iter().join(", ")
    ))
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

    let all_validators = state
        .get_validator_set()
        .await
        .wrap_err("failed to get validator set")?;
    let mut validators_that_voted = HashSet::new();

    for vote in &extended_commit_info.votes {
        let address = state
            .try_base_prefixed(vote.validator.address.as_slice())
            .await
            .wrap_err("failed to construct validator address with base prefix")?;

        ensure!(
            validators_that_voted.insert(&vote.validator.address),
            "{} voted twice",
            base64(&vote.validator.address)
        );

        total_voting_power = total_voting_power
            .checked_add(vote.validator.power.value())
            .ok_or_eyre("calculating total voting power overflowed")?;

        let signature = if vote.sig_info == Flag(tendermint::block::BlockIdFlag::Commit) {
            vote.extension_signature
                .as_ref()
                .ok_or_else(|| eyre!("vote extension signature is missing for validator {address}"))
                .and_then(|sig| {
                    Signature::try_from(sig.as_bytes()).wrap_err("failed to create signature")
                })?
        } else {
            ensure!(
                vote.vote_extension.is_empty(),
                "non-commit vote extension present for validator {address}"
            );
            ensure!(
                vote.extension_signature.is_none(),
                "non-commit extension signature present for validator {address}",
            );
            continue;
        };

        submitted_voting_power = submitted_voting_power
            .checked_add(vote.validator.power.value())
            .ok_or_eyre("calculating submitted voting power overflowed")?;

        let verification_key = &all_validators
            .get(&vote.validator.address)
            .ok_or_else(|| {
                eyre!(
                    "{} not found in validator set",
                    base64(&vote.validator.address)
                )
            })?
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
        .ok_or_eyre("failed to multiply total voting power by 2")?
        .checked_div(3)
        .ok_or_eyre("failed to divide total voting power by 3")?
        .checked_add(1)
        .ok_or_eyre("failed to add 1 from total voting power")?;
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

pub(super) async fn apply_prices_from_vote_extensions<S: StateWriteExt>(
    state: &mut S,
    extended_commit_info: &ExtendedCommitInfoWithCurrencyPairMapping,
    timestamp: Timestamp,
    height: u64,
) -> Result<()> {
    let ExtendedCommitInfoWithCurrencyPairMapping {
        extended_commit_info,
        id_to_currency_pair,
    } = extended_commit_info;

    let prices = astria_core::oracles::price_feed::utils::calculate_prices_from_vote_extensions(
        extended_commit_info,
        id_to_currency_pair,
    )
    .wrap_err("failed to calculate prices from vote extensions")?;
    for price in prices {
        let quote_price = QuotePrice {
            price: price.price(),
            block_timestamp: astria_core::Timestamp {
                seconds: timestamp.seconds,
                nanos: timestamp.nanos,
            },
            block_height: height,
        };
        state
            .put_price_for_currency_pair(price.currency_pair().clone(), quote_price)
            .await
            .wrap_err("failed to put price")?;
        debug!(
            "applied price from vote extension currency_pair=\"{}\" price={}",
            price.currency_pair(),
            price.price()
        );
        let event = abci::Event::new(
            "price_update",
            [
                ("currency_pair", price.currency_pair().to_string()),
                ("price", price.price().to_string()),
            ],
        );
        state.record(event);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        collections::BTreeMap,
        fmt::Debug,
    };

    use astria_core::{
        crypto::SigningKey,
        oracles::price_feed::{
            market_map::v2::{
                Market,
                MarketMap,
                Ticker,
            },
            oracle::v2::CurrencyPairState,
            types::v2::{
                CurrencyPair,
                CurrencyPairId,
                CurrencyPairNonce,
            },
        },
        protocol::transaction::v1::action::ValidatorUpdate,
        Timestamp,
    };
    use cnidarium::{
        Snapshot,
        StateDelta,
        TempStorage,
    };
    use tendermint::abci::types::{
        ExtendedVoteInfo,
        Validator,
        VoteInfo,
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
        oracles::price_feed::market_map::state_ext::StateWriteExt as _,
    };

    const CHAIN_ID: &str = "test-0";

    #[test]
    fn verify_vote_extension_empty_ok() {
        verify_vote_extension(vec![].into(), 100).unwrap();
    }

    #[test]
    fn verify_vote_extension_too_many_prices() {
        let vote_extension = RawOracleVoteExtension {
            prices: (0u64..=1)
                .map(|i| (i, vec![].into()))
                .collect::<BTreeMap<_, _>>(),
        };
        assert!(
            verify_vote_extension(vote_extension.encode_to_vec().into(), 1)
                .unwrap_err()
                .to_string()
                .contains(
                    "number of oracle vote extension prices exceeds max expected number of \
                     currency pairs"
                )
        );
    }

    #[test]
    fn verify_vote_extension_price_too_long() {
        let vote_extension = RawOracleVoteExtension {
            prices: (0u64..=1)
                .map(|i| (i, vec![0u8; MAXIMUM_PRICE_BYTE_LEN + 1].into()))
                .collect::<BTreeMap<_, _>>(),
        };
        assert!(
            verify_vote_extension(vote_extension.encode_to_vec().into(), 2)
                .unwrap_err()
                .to_string()
                .contains("encoded price length exceeded")
        );
    }

    fn canonical_vote_extension() -> CanonicalVoteExtension {
        let mut prices = BTreeMap::new();
        let _ = prices.insert(0, vec![].into());
        let _ = prices.insert(1, vec![].into());
        let _ = prices.insert(2, vec![].into());
        let extension = RawOracleVoteExtension {
            prices,
        }
        .encode_to_vec();
        CanonicalVoteExtension {
            extension,
            height: 1,
            round: 1,
            chain_id: CHAIN_ID.to_string(),
        }
    }

    fn extended_commit_info(round: i64, votes: Vec<ExtendedVoteInfo>) -> ExtendedCommitInfo {
        ExtendedCommitInfo {
            round: u16::try_from(round).unwrap().into(),
            votes,
        }
    }

    fn extended_vote_info_commit(
        signer: &Signer,
        canonical_vote_extension: &CanonicalVoteExtension,
    ) -> ExtendedVoteInfo {
        let message_to_sign = canonical_vote_extension.encode_length_delimited_to_vec();
        ExtendedVoteInfo {
            validator: Validator {
                address: *signer.signing_key.verification_key().address_bytes(),
                power: signer.power.into(),
            },
            sig_info: Flag(tendermint::block::BlockIdFlag::Commit),
            extension_signature: Some(
                signer
                    .signing_key
                    .sign(&message_to_sign)
                    .to_bytes()
                    .to_vec()
                    .try_into()
                    .unwrap(),
            ),
            vote_extension: canonical_vote_extension.extension.clone().into(),
        }
    }

    fn extended_vote_info_nil(signer: &Signer) -> ExtendedVoteInfo {
        ExtendedVoteInfo {
            validator: Validator {
                address: *signer.signing_key.verification_key().address_bytes(),
                power: signer.power.into(),
            },
            sig_info: Flag(tendermint::block::BlockIdFlag::Nil),
            extension_signature: None,
            vote_extension: vec![].into(),
        }
    }

    fn height(message: &CanonicalVoteExtension) -> u64 {
        u64::try_from(message.height).unwrap()
    }

    fn last_commit<'a, T: IntoIterator<Item = &'a Signer>>(signers: T, round: i64) -> CommitInfo {
        let votes = signers
            .into_iter()
            .map(|signer| VoteInfo {
                validator: Validator {
                    address: signer.signing_key.address_bytes(),
                    power: signer.power.into(),
                },
                sig_info: Flag(tendermint::block::BlockIdFlag::Commit),
            })
            .collect();
        CommitInfo {
            round: u16::try_from(round).unwrap().into(),
            votes,
        }
    }

    fn pair_0() -> (CurrencyPair, CurrencyPairId) {
        ("ETH/USD".parse().unwrap(), CurrencyPairId::new(0))
    }

    fn pair_1() -> (CurrencyPair, CurrencyPairId) {
        ("BTC/USD".parse().unwrap(), CurrencyPairId::new(1))
    }

    fn pair_2() -> (CurrencyPair, CurrencyPairId) {
        ("TIA/USD".parse().unwrap(), CurrencyPairId::new(2))
    }

    struct Signer {
        signing_key: SigningKey,
        power: u8,
    }

    impl Signer {
        fn new(signing_key_bytes: [u8; 32], power: u8) -> Self {
            Self {
                signing_key: SigningKey::from(signing_key_bytes),
                power,
            }
        }
    }

    struct Fixture {
        signer_a: Signer,
        signer_b: Signer,
        signer_c: Signer,
        state: StateDelta<Snapshot>,
        _storage: TempStorage,
    }

    impl Fixture {
        async fn new() -> Self {
            let signer_a = Signer::new([0; 32], 6);
            let signer_b = Signer::new([1; 32], 2);
            let signer_c = Signer::new([2; 32], 1);

            let storage = TempStorage::new().await.unwrap();
            let mut state = StateDelta::new(storage.latest_snapshot());
            state
                .put_chain_id_and_revision_number(CHAIN_ID.try_into().unwrap())
                .unwrap();
            let validator_set = ValidatorSet::new_from_updates(vec![
                ValidatorUpdate {
                    power: signer_a.power.into(),
                    verification_key: signer_a.signing_key.verification_key(),
                },
                ValidatorUpdate {
                    power: signer_b.power.into(),
                    verification_key: signer_b.signing_key.verification_key(),
                },
                ValidatorUpdate {
                    power: signer_c.power.into(),
                    verification_key: signer_c.signing_key.verification_key(),
                },
            ]);
            state.put_validator_set(validator_set).unwrap();
            state.put_base_prefix("astria".to_string()).unwrap();

            let mut market_map = MarketMap {
                markets: IndexMap::new(),
            };

            for (pair, pair_id) in [pair_0(), pair_1(), pair_2()] {
                let pair_state = CurrencyPairState {
                    price: Some(QuotePrice {
                        price: Price::new(123),
                        block_timestamp: Timestamp {
                            seconds: 4,
                            nanos: 5,
                        },
                        block_height: 1,
                    }),
                    nonce: CurrencyPairNonce::new(1),
                    id: pair_id,
                };
                state
                    .put_currency_pair_state(pair.clone(), pair_state)
                    .unwrap();
                market_map.markets.insert(
                    pair.to_string(),
                    Market {
                        ticker: Ticker {
                            currency_pair: pair,
                            decimals: 6,
                            min_provider_count: 0,
                            enabled: true,
                            metadata_json: String::new(),
                        },
                        provider_configs: Vec::new(),
                    },
                );
            }
            state.put_num_currency_pairs(3).unwrap();
            state.put_market_map(market_map).unwrap();

            Self {
                signer_a,
                signer_b,
                signer_c,
                state,
                _storage: storage,
            }
        }
    }

    #[track_caller]
    fn assert_err_contains<T: Debug, E: ToString>(result: Result<T, E>, messages: &[&str]) {
        let actual_message = result.unwrap_err().to_string();
        for message in messages {
            assert!(
                actual_message.contains(message),
                "error expected to contain `{message}`, but the actual error message is \
                 `{actual_message}`"
            );
        }
    }

    /// Should fail validation if any validator votes more than once.
    #[tokio::test]
    async fn validate_vote_extensions_repeated_voter() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_a, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["voted twice"],
        );
    }

    /// Should fail validation if any of the votes is a `Commit` type but doesn't include a
    /// signature.
    #[tokio::test]
    async fn validate_vote_extensions_missing_sig() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let mut bad_vote = extended_vote_info_commit(&signer_a, &message);
        bad_vote.extension_signature = None;
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            bad_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["vote extension signature is missing for validator"],
        );
    }

    /// Should fail validation if any of the votes is not a `Commit` type and also includes a vote
    /// extension.
    #[tokio::test]
    async fn validate_vote_extensions_nil_with_extension() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let mut bad_vote = extended_vote_info_nil(&signer_a);
        bad_vote.vote_extension = vec![1_u8].into();
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            bad_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["non-commit vote extension present for validator"],
        );
    }

    /// Should fail validation if any of the votes is not a `Commit` type and also includes a
    /// signature.
    #[tokio::test]
    async fn validate_vote_extensions_nil_with_signature() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let mut bad_vote = extended_vote_info_nil(&signer_a);
        bad_vote.extension_signature = Some(vec![1_u8; 64].try_into().unwrap());
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            bad_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["non-commit extension signature present for validator"],
        );
    }

    /// Should fail validation if any of the votes is a `Commit` type with a signature by a key
    /// not in the validator set.
    #[tokio::test]
    async fn validate_vote_extensions_unknown_signer() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let unknown_signer = Signer::new([9; 32], 10);
        let bad_vote = extended_vote_info_commit(&unknown_signer, &message);
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_a, &message),
            bad_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["not found in validator set"],
        );
    }

    /// Should fail validation if any of the votes is a `Commit` type with an invalid signature.
    #[tokio::test]
    async fn validate_vote_extensions_invalid_signature() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let mut bad_vote = extended_vote_info_commit(&signer_a, &message);
        bad_vote.extension_signature = Some(vec![0; 64].try_into().unwrap());
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            bad_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["failed to verify signature for vote extension"],
        );
    }

    /// Should fail validation if there are no votes.
    #[tokio::test]
    async fn validate_vote_extensions_no_votes() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let extended_commit_info = extended_commit_info(1, vec![]);
        assert_err_contains(
            validate_vote_extensions(&state, 2, &extended_commit_info).await,
            &["total voting power is zero"],
        );
    }

    /// Should fail validation if the total power of `Commit` type votes is less than 2/3 of the
    /// total power of all votes.
    #[tokio::test]
    async fn validate_vote_extensions_insufficient_voting_power() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        // Signer A has 2/3 voting power, and sends a nil vote.
        let nil_vote = extended_vote_info_nil(&signer_a);
        let votes = vec![
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
            nil_vote,
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        assert_err_contains(
            validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info).await,
            &["submitted voting power is less than required voting power"],
        );
    }

    #[tokio::test]
    async fn validate_vote_extensions_ok() {
        let Fixture {
            signer_c,
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![extended_vote_info_commit(&signer_c, &message)];
        let extended_commit_info = extended_commit_info(message.round, votes);
        validate_vote_extensions(&state, height(&message) + 1, &extended_commit_info)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn validate_against_last_commit_wrong_round() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round + 1, votes);
        let last_commit = last_commit([&signer_a, &signer_b, &signer_c], message.round);
        assert_err_contains(
            validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info),
            &["last commit round does not match extended commit round"],
        );
    }

    #[tokio::test]
    async fn validate_against_last_commit_num_votes_mismatch() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        let last_commit = last_commit([&signer_a, &signer_b, &signer_c], message.round);
        assert_err_contains(
            validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info),
            &["last commit votes length does not match extended commit votes length"],
        );
    }

    #[tokio::test]
    async fn validate_against_last_commit_voter_address_mismatch() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        let bad_signer = Signer::new([9; 32], signer_c.power);
        let last_commit = last_commit([&signer_a, &signer_b, &bad_signer], message.round);
        assert_err_contains(
            validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info),
            &["last commit vote address does not match extended commit vote address"],
        );
    }

    #[tokio::test]
    async fn validate_against_last_commit_voter_power_mismatch() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        let bad_signer = Signer {
            signing_key: signer_c.signing_key.clone(),
            power: signer_c.power.checked_add(1).unwrap(),
        };
        let last_commit = last_commit([&signer_a, &signer_b, &bad_signer], message.round);
        assert_err_contains(
            validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info),
            &["last commit vote power does not match extended commit vote power"],
        );
    }

    #[tokio::test]
    async fn validate_against_last_commit_sig_info_mismatch() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        let mut last_commit = last_commit([&signer_a, &signer_b, &signer_c], message.round);
        // Change the type of the final vote's sig info to create a mismatch.
        last_commit.votes.last_mut().unwrap().sig_info = Flag(tendermint::block::BlockIdFlag::Nil);
        assert_err_contains(
            validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info),
            &["last commit vote sig info does not match extended commit vote sig info"],
        );
    }

    #[tokio::test]
    async fn validate_against_last_commit_ok() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            ..
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info = extended_commit_info(message.round, votes);
        let last_commit = last_commit([&signer_a, &signer_b, &signer_c], message.round);
        validate_extended_commit_against_last_commit(&last_commit, &extended_commit_info).unwrap();
    }

    // When constructing the mapping, if an ID doesn't have a corresponding CurrencyPair in storage,
    // it should just get omitted from the mapping rather than triggering an error.
    #[tokio::test]
    async fn get_id_to_currency_pair_mapping_should_allow_missing_id() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        // No mapping for IDs 3 and 4.
        let ids_missing_pairs = HashSet::from_iter([0, 1, 2, 3, 4]);
        let id_to_currency_pairs = get_id_to_currency_pair(&state, ids_missing_pairs.clone())
            .await
            .unwrap();
        assert_eq!(3, id_to_currency_pairs.len());
        assert!(id_to_currency_pairs.contains_key(&CurrencyPairId::new(0)));
        assert!(id_to_currency_pairs.contains_key(&CurrencyPairId::new(1)));
        assert!(id_to_currency_pairs.contains_key(&CurrencyPairId::new(2)));

        // Check that validation using this same set of IDs passes.
        validate_id_to_currency_pair_mapping(&state, ids_missing_pairs, &id_to_currency_pairs)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn validate_id_to_currency_pair_mapping_missing_pair() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let ids_missing_pairs = HashSet::from_iter([0]);
        let id_to_currency_pairs = get_id_to_currency_pair(&state, ids_missing_pairs)
            .await
            .unwrap();
        let all_ids = HashSet::from_iter([0, 1, 2]);
        assert_err_contains(
            validate_id_to_currency_pair_mapping(&state, all_ids, &id_to_currency_pairs).await,
            &[
                "failed to validate currency pair mapping:",
                "missing (expected `CurrencyPairInfo { currency_pair: BTC/USD, decimals: 6 }` for \
                 id 1)",
                "missing (expected `CurrencyPairInfo { currency_pair: TIA/USD, decimals: 6 }` for \
                 id 2)",
            ],
        );
    }

    #[tokio::test]
    async fn validate_id_to_currency_pair_mapping_extra_pair() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let ids_extra_pair = HashSet::from_iter([0, 1, 2]);
        let id_to_currency_pairs = get_id_to_currency_pair(&state, ids_extra_pair)
            .await
            .unwrap();
        let all_ids = HashSet::from_iter([0]);
        assert_err_contains(
            validate_id_to_currency_pair_mapping(&state, all_ids, &id_to_currency_pairs).await,
            &[
                "failed to validate currency pair mapping:",
                "unexpected (got `CurrencyPairInfo { currency_pair: BTC/USD, decimals: 6 }` for \
                 id 1)",
                "unexpected (got `CurrencyPairInfo { currency_pair: TIA/USD, decimals: 6 }` for \
                 id 2)",
            ],
        );
    }

    #[tokio::test]
    async fn validate_id_to_currency_pair_mapping_pair_mismatch() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let all_ids = HashSet::from_iter([0, 1, 2]);
        let mut id_to_currency_pairs = get_id_to_currency_pair(&state, all_ids.clone())
            .await
            .unwrap();
        id_to_currency_pairs
            .get_mut(&CurrencyPairId::new(0))
            .unwrap()
            .currency_pair = "ABC/DEF".parse().unwrap();
        id_to_currency_pairs
            .get_mut(&CurrencyPairId::new(1))
            .unwrap()
            .currency_pair = "GHI/JKL".parse().unwrap();
        assert_err_contains(
            validate_id_to_currency_pair_mapping(&state, all_ids, &id_to_currency_pairs).await,
            &[
                "failed to validate currency pair mapping:",
                "mismatch (expected `CurrencyPairInfo { currency_pair: ETH/USD, decimals: 6 }` \
                 but got `CurrencyPairInfo { currency_pair: ABC/DEF, decimals: 6 }` for id 0)",
                "mismatch (expected `CurrencyPairInfo { currency_pair: BTC/USD, decimals: 6 }` \
                 but got `CurrencyPairInfo { currency_pair: GHI/JKL, decimals: 6 }` for id 1)",
            ],
        );
    }

    #[tokio::test]
    async fn validate_id_to_currency_pair_mapping_ok() {
        let Fixture {
            state,
            _storage,
            ..
        } = Fixture::new().await;

        let all_ids = HashSet::from_iter([0, 1, 2]);
        let id_to_currency_pairs = get_id_to_currency_pair(&state, all_ids.clone())
            .await
            .unwrap();
        // Ensure the random order of `all_ids` has no bearing on the internal equality check.
        let first_element = *all_ids.iter().next().unwrap();
        loop {
            let ids: HashSet<u64> = all_ids.iter().copied().collect();
            if *ids.iter().next().unwrap() != first_element {
                validate_id_to_currency_pair_mapping(&state, ids, &id_to_currency_pairs)
                    .await
                    .unwrap();
                return;
            }
        }
    }

    #[tokio::test]
    async fn prepare_proposal_and_validate_proposal() {
        let Fixture {
            signer_a,
            signer_b,
            signer_c,
            state,
            _storage,
        } = Fixture::new().await;

        let message = canonical_vote_extension();
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_b, &message),
            extended_vote_info_commit(&signer_c, &message),
        ];
        let extended_commit_info_with_currency_pair_mapping = ProposalHandler::prepare_proposal(
            &state,
            height(&message) + 1,
            extended_commit_info(message.round, votes),
        )
        .await
        .unwrap();

        let last_commit = last_commit([&signer_a, &signer_b, &signer_c], message.round);
        ProposalHandler::validate_proposal(
            &state,
            height(&message) + 1,
            &last_commit,
            &extended_commit_info_with_currency_pair_mapping,
        )
        .await
        .unwrap();

        // unsorted extended commit info should fail
        let votes = vec![
            extended_vote_info_commit(&signer_a, &message),
            extended_vote_info_commit(&signer_c, &message),
            extended_vote_info_commit(&signer_b, &message),
        ];
        let unsorted_extended_commit_info = ExtendedCommitInfoWithCurrencyPairMapping {
            extended_commit_info: extended_commit_info(message.round, votes),
            id_to_currency_pair: extended_commit_info_with_currency_pair_mapping
                .id_to_currency_pair
                .clone(),
        };

        assert_err_contains(
            ProposalHandler::validate_proposal(
                &state,
                height(&message) + 1,
                &last_commit,
                &unsorted_extended_commit_info,
            )
            .await,
            &["last commit vote address does not match extended commit vote address"],
        );
    }
}
