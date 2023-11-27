use std::{
    collections::HashMap,
    hash::Hash,
};

use tokio::sync::{
    mpsc,
    oneshot,
};
use tonic::Response;

use crate::{
    generated::block_relay::v1alpha1::{
        top_of_block_relay_server::TopOfBlockRelay,
        GetBidRequest,
        GetBidResponse,
        GetBundleRequest,
        GetBundleResponse,
    },
    native::{
        composer::block_relay::v1alpha1::bid::SignedBundle,
        sequencer::v1alpha1::Address,
    },
};

pub(super) struct TopOfBlockRelayService {
    // Channel to request the current best bid from the top of block relay actor
    // TODO: needs a better name
    best_bid_request_tx: mpsc::Sender<oneshot::Sender<SignedBundle>>,
    // Bids that have been committed to by proposers
    pending_commitment: HashMap<Address, SignedBundle>,
}

impl TopOfBlockRelayService {
    pub fn new(best_bid_request_tx: mpsc::Sender<oneshot::Sender<SignedBundle>>) -> Self {
        Self {
            best_bid_request_tx,
            pending_commitment: HashMap::new(),
        }
    }
}

#[tonic::async_trait]
impl TopOfBlockRelay for TopOfBlockRelayService {
    async fn get_bid(
        &self,
        request: tonic::Request<GetBidRequest>,
    ) -> Result<tonic::Response<GetBidResponse>, tonic::Status> {
        // get the best bid
        let (bundle_tx, bundle_rx) = oneshot::channel();
        self.best_bid_request_tx
            .send(bundle_tx)
            .await
            .map_err(|_| tonic::Status::internal("failed to request best bid"))?;

        let signed_bundle = bundle_rx
            .await
            .map_err(|_| tonic::Status::internal("failed to receive best bid"))?;

        let bid = signed_bundle.bundle.to_opaque_bid();

        // save the pending commitment under the proposer's address

        let rsp = GetBidResponse {
            bid: Some(bid.into_raw()),
        };
        Ok(Response::new(rsp))
    }

    async fn get_bundle(
        &self,
        request: tonic::Request<GetBundleRequest>,
    ) -> Result<tonic::Response<GetBundleResponse>, tonic::Status> {
        todo!("construct the payload_hash for the requested bid");

        todo!("verify proposer's commitment to the payload_hash");

        todo!("flush pending commitments");
        // Ok(Response::new(rsp))
    }
}
