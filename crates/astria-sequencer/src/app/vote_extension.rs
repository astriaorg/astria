use std::collections::HashMap;

use anyhow::{
    ensure,
    Context as _,
};
use astria_core::{
    generated::slinky::{
        abci::v1::OracleVoteExtension as RawOracleVoteExtension,
        service::v1::{
            oracle_client::OracleClient,
            QueryPricesRequest,
            QueryPricesResponse,
        },
    },
    slinky::abci::v1::OracleVoteExtension,
};
use prost::Message as _;
use tendermint::abci;
use tonic::transport::Channel;

use crate::{
    oracle::currency_pair_strategy::DefaultCurrencyPairStrategy,
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
        vote_extension: abci::request::VerifyVoteExtension,
        is_proposal_phase: bool,
    ) -> anyhow::Result<abci::response::VerifyVoteExtension> {
        let oracle_vote_extension = RawOracleVoteExtension::decode(vote_extension.vote_extension)?;

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

        Ok(abci::response::VerifyVoteExtension::Accept)
    }
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
        let price = price_string.parse::<u128>()?;

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
