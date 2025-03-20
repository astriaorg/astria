use std::{
    str::FromStr,
    sync::Arc,
};

use astria_core::{
    generated::price_feed::{
        marketmap::v2::{
            query_server::Query as MarketMapQueryService,
            LastUpdatedRequest,
            LastUpdatedResponse,
            MarketMapRequest,
            MarketMapResponse,
            MarketRequest,
            MarketResponse,
            ParamsRequest,
            ParamsResponse,
        },
        oracle::v2::{
            query_server::Query as OracleService,
            GetAllCurrencyPairsRequest,
            GetAllCurrencyPairsResponse,
            GetCurrencyPairMappingRequest,
            GetCurrencyPairMappingResponse,
            GetPriceRequest,
            GetPriceResponse,
            GetPricesRequest,
            GetPricesResponse,
        },
    },
    oracles::price_feed::types::v2::CurrencyPair,
};
use cnidarium::Storage;
use futures::{
    TryFutureExt as _,
    TryStreamExt as _,
};
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::instrument;

use crate::{
    app::StateReadExt as _,
    oracles::price_feed::{
        market_map::state_ext::StateReadExt as _,
        oracle::state_ext::{
            CurrencyPairWithId,
            StateReadExt as _,
        },
    },
};

pub(crate) struct SequencerServer {
    storage: Storage,
}

impl SequencerServer {
    pub(crate) fn new(storage: Storage) -> Self {
        Self {
            storage,
        }
    }
}

#[async_trait::async_trait]
impl MarketMapQueryService for SequencerServer {
    #[instrument(skip_all)]
    async fn market_map(
        self: Arc<Self>,
        _request: Request<MarketMapRequest>,
    ) -> Result<Response<MarketMapResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let market_map = snapshot.get_market_map().await.map_err(|e| {
            Status::internal(format!(
                "failed to get block market map from storage: {e:#}"
            ))
        })?;
        let last_updated = snapshot
            .get_market_map_last_updated_height()
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get block market map last updated height from storage: {e:#}"
                ))
            })?;
        let chain_id = snapshot
            .get_chain_id()
            .await
            .map_err(|e| Status::internal(format!("failed to get chain id from storage: {e:#}")))?;

        Ok(Response::new(MarketMapResponse {
            market_map: market_map
                .map(astria_core::oracles::price_feed::market_map::v2::MarketMap::into_raw),
            last_updated,
            chain_id: chain_id.to_string(),
        }))
    }

    #[instrument(skip_all)]
    async fn market(
        self: Arc<Self>,
        request: Request<MarketRequest>,
    ) -> Result<Response<MarketResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let market_map = snapshot
            .get_market_map()
            .await
            .map_err(|e| Status::internal(format!("error getting market map from storage: {e:#}")))?
            .ok_or_else(|| Status::not_found("market map not stored"))?;
        let raw_currency_pair = request
            .into_inner()
            .currency_pair
            .ok_or_else(|| Status::invalid_argument("`currency_pair` must be set"))?;
        let currency_pair = CurrencyPair::try_from_raw(raw_currency_pair)
            .map_err(|e| Status::invalid_argument(format!("invalid `currency_pair`: {e:#}")))?
            .to_string();
        let market = market_map
            .markets
            .get(&currency_pair)
            .cloned()
            .ok_or_else(|| Status::not_found(format!("`{currency_pair}` not in market map")))?;

        Ok(Response::new(MarketResponse {
            market: Some(market.into_raw()),
        }))
    }

    #[instrument(skip_all)]
    async fn last_updated(
        self: Arc<Self>,
        _request: Request<LastUpdatedRequest>,
    ) -> Result<Response<LastUpdatedResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let last_updated = snapshot
            .get_market_map_last_updated_height()
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get block market map last updated height from storage: {e:#}"
                ))
            })?;

        Ok(Response::new(LastUpdatedResponse {
            last_updated,
        }))
    }

    #[instrument(skip_all)]
    async fn params(
        self: Arc<Self>,
        _request: Request<ParamsRequest>,
    ) -> Result<Response<ParamsResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let params = snapshot.get_params().await.map_err(|e| {
            Status::internal(format!("failed to get block params from storage: {e:#}"))
        })?;

        Ok(Response::new(ParamsResponse {
            params: params.map(astria_core::oracles::price_feed::market_map::v2::Params::into_raw),
        }))
    }
}

#[async_trait::async_trait]
impl OracleService for SequencerServer {
    #[instrument(skip_all)]
    async fn get_all_currency_pairs(
        self: Arc<Self>,
        _request: Request<GetAllCurrencyPairsRequest>,
    ) -> Result<Response<GetAllCurrencyPairsResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let currency_pairs = snapshot
            .currency_pairs()
            .map_ok(CurrencyPair::into_raw)
            .try_collect()
            .map_err(|err| {
                Status::internal(format!(
                    "failed to get all currency pairs from storage: {err:#}"
                ))
            })
            .await?;
        Ok(Response::new(GetAllCurrencyPairsResponse {
            currency_pairs,
        }))
    }

    #[instrument(skip_all)]
    async fn get_price(
        self: Arc<Self>,
        request: Request<GetPriceRequest>,
    ) -> Result<Response<GetPriceResponse>, Status> {
        let request = request.into_inner();

        let currency_pair = request
            .currency_pair
            .parse()
            .map_err(|e| Status::invalid_argument(format!("currency pair is invalid: {e:#}")))?;

        let snapshot = self.storage.latest_snapshot();
        let Some(state) = snapshot
            .get_currency_pair_state(&currency_pair)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get currency pair state from storage: {e:#}"
                ))
            })?
        else {
            return Err(Status::not_found("currency pair state not found"));
        };

        let Some(market_map) = snapshot.get_market_map().await.map_err(|e| {
            Status::internal(format!(
                "failed to get block market map from storage: {e:#}"
            ))
        })?
        else {
            return Err(Status::internal("market map not found"));
        };

        let Some(market) = market_map.markets.get(&currency_pair.to_string()) else {
            return Err(Status::not_found(format!(
                "market not found for {currency_pair}"
            )));
        };

        Ok(Response::new(GetPriceResponse {
            price: state
                .price
                .map(astria_core::oracles::price_feed::oracle::v2::QuotePrice::into_raw),
            nonce: state.nonce.get(),
            id: state.id.get(),
            decimals: market.ticker.decimals.into(),
        }))
    }

    #[instrument(skip_all)]
    async fn get_prices(
        self: Arc<Self>,
        request: Request<GetPricesRequest>,
    ) -> Result<Response<GetPricesResponse>, Status> {
        let request = request.into_inner();
        let currency_pairs = request
            .currency_pair_ids
            .into_iter()
            .map(|s| {
                CurrencyPair::from_str(&s).map_err(|e| {
                    Status::invalid_argument(format!("invalid currency pair id `{s}`: {e:#}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let snapshot = self.storage.latest_snapshot();
        let Some(market_map) = snapshot.get_market_map().await.map_err(|e| {
            Status::internal(format!(
                "failed to get block market map from storage: {e:#}"
            ))
        })?
        else {
            return Err(Status::internal("market map not found"));
        };

        let mut prices = Vec::new();
        for currency_pair in currency_pairs {
            let Some(state) = snapshot
                .get_currency_pair_state(&currency_pair)
                .await
                .map_err(|e| {
                    Status::internal(format!("failed to get state from storage: {e:#}"))
                })?
            else {
                return Err(Status::not_found(format!(
                    "currency pair state for {currency_pair} not found"
                )));
            };

            let Some(market) = market_map.markets.get(&currency_pair.to_string()) else {
                return Err(Status::not_found(format!(
                    "market not found for {currency_pair}"
                )));
            };

            prices.push(GetPriceResponse {
                price: state
                    .price
                    .map(astria_core::oracles::price_feed::oracle::v2::QuotePrice::into_raw),
                nonce: state.nonce.get(),
                id: state.id.get(),
                decimals: market.ticker.decimals.into(),
            });
        }
        Ok(Response::new(GetPricesResponse {
            prices,
        }))
    }

    #[instrument(skip_all)]
    async fn get_currency_pair_mapping(
        self: Arc<Self>,
        _request: Request<GetCurrencyPairMappingRequest>,
    ) -> Result<Response<GetCurrencyPairMappingResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let stream = snapshot.currency_pairs_with_ids();
        let currency_pair_mapping = stream
            .map_ok(
                |CurrencyPairWithId {
                     id,
                     currency_pair,
                 }| (id, currency_pair.into_raw()),
            )
            .try_collect()
            .map_err(|err| {
                Status::internal(format!(
                    "failed to get currency pair mapping from storage: {err:#}"
                ))
            })
            .await?;
        Ok(Response::new(GetCurrencyPairMappingResponse {
            currency_pair_mapping,
        }))
    }
}
