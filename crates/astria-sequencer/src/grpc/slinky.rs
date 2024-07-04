use std::{
    str::FromStr,
    sync::Arc,
};

use astria_core::{
    generated::slinky::{
        marketmap::v1::{
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
        oracle::v1::{
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
    slinky::types::v1::CurrencyPair,
};
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::instrument;

use crate::{
    slinky::{
        marketmap::state_ext::StateReadExt as _,
        oracle::state_ext::StateReadExt as _,
    },
    state_ext::StateReadExt as _,
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
            Status::internal(format!("failed to get block market map from storage: {e}"))
        })?;
        let last_updated = snapshot
            .get_market_map_last_updated_height()
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get block market map last updated height from storage: {e}"
                ))
            })?;
        let chain_id = snapshot
            .get_chain_id()
            .await
            .map_err(|e| Status::internal(format!("failed to get chain id from storage: {e}")))?;

        Ok(Response::new(MarketMapResponse {
            market_map: market_map.map(astria_core::slinky::market_map::v1::MarketMap::into_raw),
            last_updated,
            chain_id: chain_id.to_string(), // TODO: is this the right chain id?
        }))
    }

    #[instrument(skip_all)]
    async fn market(
        self: Arc<Self>,
        _request: Request<MarketRequest>,
    ) -> Result<Response<MarketResponse>, Status> {
        // TODO
        Ok(Response::new(MarketResponse::default()))
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
                    "failed to get block market map last updated height from storage: {e}"
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
            Status::internal(format!("failed to get block params from storage: {e}"))
        })?;

        Ok(Response::new(ParamsResponse {
            params: params.map(astria_core::slinky::market_map::v1::Params::into_raw),
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
        let currency_pairs = snapshot.get_all_currency_pairs().await.map_err(|e| {
            Status::internal(format!(
                "failed to get all currency pairs from storage: {e}"
            ))
        })?;
        Ok(Response::new(GetAllCurrencyPairsResponse {
            currency_pairs: currency_pairs
                .into_iter()
                .map(CurrencyPair::into_raw)
                .collect(),
        }))
    }

    #[instrument(skip_all)]
    async fn get_price(
        self: Arc<Self>,
        request: Request<GetPriceRequest>,
    ) -> Result<Response<GetPriceResponse>, Status> {
        let request = request.into_inner();
        let Some(currency_pair) = request.currency_pair else {
            return Err(Status::invalid_argument("currency pair is required"));
        };
        let currency_pair = CurrencyPair::from_raw(currency_pair);
        let snapshot = self.storage.latest_snapshot();
        let Some(state) = snapshot
            .get_currency_pair_state(&currency_pair)
            .await
            .map_err(|e| Status::internal(format!("failed to get state from storage: {e}")))?
        else {
            return Err(Status::not_found("currency pair state not found"));
        };

        Ok(Response::new(GetPriceResponse {
            price: Some(state.price.into_raw()),
            nonce: state.nonce,
            id: state.id,
            decimals: 0, // TODO: get this from the marketmap
        }))
    }

    #[instrument(skip_all)]
    async fn get_prices(
        self: Arc<Self>,
        request: Request<GetPricesRequest>,
    ) -> Result<Response<GetPricesResponse>, Status> {
        let request = request.into_inner();
        let currency_pairs = match request
            .currency_pair_ids
            .into_iter()
            .map(|s| CurrencyPair::from_str(&s))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(currency_pairs) => currency_pairs,
            Err(e) => {
                return Err(Status::invalid_argument(format!(
                    "invalid currency pair id: {e}"
                )));
            }
        };

        let snapshot = self.storage.latest_snapshot();
        let mut prices = Vec::new();
        for currency_pair in currency_pairs {
            let Some(state) = snapshot
                .get_currency_pair_state(&currency_pair)
                .await
                .map_err(|e| Status::internal(format!("failed to get state from storage: {e}")))?
            else {
                return Err(Status::not_found("currency pair state not found"));
            };
            prices.push(GetPriceResponse {
                price: Some(state.price.into_raw()),
                nonce: state.nonce,
                id: state.id,
                decimals: 0, // TODO: get this from the marketmap
            })
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
        let currency_pair_mapping = snapshot.get_currency_pair_mapping().await.map_err(|e| {
            Status::internal(format!(
                "failed to get currency pair mapping from storage: {e}"
            ))
        })?;
        let currency_pair_mapping = currency_pair_mapping
            .into_iter()
            .map(|(k, v)| (k, v.into_raw()))
            .collect();

        Ok(Response::new(GetCurrencyPairMappingResponse {
            currency_pair_mapping,
        }))
    }
}
