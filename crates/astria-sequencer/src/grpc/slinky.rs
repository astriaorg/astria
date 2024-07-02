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
        request: Request<MarketMapRequest>,
    ) -> Result<Response<MarketMapResponse>, Status> {
        Ok(Response::new(MarketMapResponse::default()))
    }

    #[instrument(skip_all)]
    async fn market(
        self: Arc<Self>,
        request: Request<MarketRequest>,
    ) -> Result<Response<MarketResponse>, Status> {
        Ok(Response::new(MarketResponse::default()))
    }

    #[instrument(skip_all)]
    async fn last_updated(
        self: Arc<Self>,
        request: Request<LastUpdatedRequest>,
    ) -> Result<Response<LastUpdatedResponse>, Status> {
        Ok(Response::new(LastUpdatedResponse::default()))
    }

    #[instrument(skip_all)]
    async fn params(
        self: Arc<Self>,
        request: Request<ParamsRequest>,
    ) -> Result<Response<ParamsResponse>, Status> {
        Ok(Response::new(ParamsResponse::default()))
    }
}
