use std::collections::HashMap;

use proto::generated::block_relay::v1alpha1::{
    top_of_block_service_server::TopOfBlockService,
    GetTopOfBlockBidRequest,
    GetTopOfBlockBidResponse,
    GetTopOfBlockPayloadRequest,
    GetTopOfBlockPayloadResponse,
};

struct TopOfBlockRelay {
    current_bids: HashMap<u64, GetTopOfBlockBidResponse>,
}

#[tonic::async_trait]
impl TopOfBlockService for TopOfBlockRelay {
    async fn get_top_of_block_bid(
        &self,
        request: tonic::Request<GetTopOfBlockBidRequest>,
    ) -> Result<tonic::Response<GetTopOfBlockBidResponse>, tonic::Status> {
        unimplemented!()
    }

    async fn get_top_of_block_payload(
        &self,
        request: tonic::Request<GetTopOfBlockPayloadRequest>,
    ) -> Result<tonic::Response<GetTopOfBlockPayloadResponse>, tonic::Status> {
        unimplemented!()
    }
}
