use std::collections::HashMap;

use anyhow::Context as _;
use astria_core::generated::slinky::{
    abci::v1::OracleVoteExtension,
    service::v1::{
        oracle_client::OracleClient,
        QueryPricesRequest,
        QueryPricesResponse,
    },
    types::v1::CurrencyPair,
};
use prost::Message as _;
use tendermint::abci;
use tonic::transport::Channel;

pub(crate) struct Handler {
    // gRPC client for the slinky oracle sidecar.
    oracle_client: OracleClient<Channel>,
}

impl Handler {
    pub(crate) fn new(oracle_client: OracleClient<Channel>) -> Self {
        Self {
            oracle_client,
        }
    }

    pub(crate) async fn extend_vote(&mut self) -> anyhow::Result<abci::response::ExtendVote> {
        // TODO: use oracle client timeout
        let prices = match self.oracle_client.prices(QueryPricesRequest {}).await {
            Ok(prices) => prices.into_inner(),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "failed to get prices from oracle sidecar"
                );
                return Ok(abci::response::ExtendVote {
                    vote_extension: vec![].into(),
                });
            }
        };

        let oracle_vote_extension = self
            .transform_oracle_service_prices(prices)
            .context("failed to transform oracle service prices")?;
        Ok(abci::response::ExtendVote {
            // TODO: what codec does skip use for this? does it matter here?
            // don't think so but good to check
            vote_extension: oracle_vote_extension.encode_to_vec().into(),
        })
    }

    pub(crate) async fn verify_vote_extension(
        &mut self,
        vote_extension: abci::request::VerifyVoteExtension,
    ) -> anyhow::Result<abci::response::VerifyVoteExtension> {
        // TODO: verify the vote extension based on slinky rules
        let _oracle_vote_extension = OracleVoteExtension::decode(vote_extension.vote_extension)?;
        Ok(abci::response::VerifyVoteExtension::Accept)
    }

    // see https://github.com/skip-mev/slinky/blob/158cde8a4b774ac4eec5c6d1a2c16de6a8c6abb5/abci/ve/vote_extension.go#L290
    fn transform_oracle_service_prices(
        &self,
        prices: QueryPricesResponse,
    ) -> anyhow::Result<OracleVoteExtension> {
        for (currency_pair_id, price_string) in prices.prices {
            let _currency_pair = currency_pair_from_string(&currency_pair_id)?;
            let _price = price_string.parse::<u128>()?;

            // TODO: oracle module state
        }

        Ok(OracleVoteExtension {
            prices: HashMap::default(),
        })
    }
}

fn currency_pair_from_string(s: &str) -> anyhow::Result<CurrencyPair> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("invalid currency pair string: {}", s);
    }
    Ok(CurrencyPair {
        base: parts[0].to_string(),
        quote: parts[1].to_string(),
    })
}
