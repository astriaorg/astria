use std::sync::Arc;

use astria_core::generated::slinky::marketmap::v1::{
    query_server::Query as MarketMapQueryService,
    LastUpdatedRequest,
    LastUpdatedResponse,
    MarketMapRequest,
    MarketMapResponse,
    MarketRequest,
    MarketResponse,
    ParamsRequest,
    ParamsResponse,
};
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::instrument;

use crate::{
    slinky::state_ext::StateReadExt as _,
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
