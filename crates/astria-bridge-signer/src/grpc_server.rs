use std::sync::Arc;

use astria_core::generated::astria::signer::v1::{
    frost_participant_service_server::FrostParticipantService,
    GetVerifyingShareRequest,
    GetVerifyingShareResponse,
    Part1Request,
    Part1Response,
    Part2Request,
    Part2Response,
};
use tonic::{
    async_trait,
    Request,
    Response,
    Status,
};

use crate::metrics::Metrics;

pub struct Server {
    metrics: &'static Metrics,
}

impl Server {
    pub fn new(metrics: &'static Metrics) -> Self {
        Self {
            metrics,
        }
    }
}

#[async_trait]
impl FrostParticipantService for Server {
    async fn get_verifying_share(
        self: Arc<Self>,
        request: Request<GetVerifyingShareRequest>,
    ) -> Result<Response<GetVerifyingShareResponse>, Status> {
        todo!()
    }

    async fn part1(
        self: Arc<Self>,
        request: Request<Part1Request>,
    ) -> Result<Response<Part1Response>, Status> {
        todo!()
    }

    async fn part2(
        self: Arc<Self>,
        request: Request<Part2Request>,
    ) -> Result<Response<Part2Response>, Status> {
        todo!()
    }
}
