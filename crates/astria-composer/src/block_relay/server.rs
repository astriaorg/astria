use std::net::SocketAddr;

use color_eyre::eyre::{
    self,
    Context,
};
use proto::{
    generated::block_relay::v1alpha1::top_of_block_relay_server::TopOfBlockRelayServer,
    native::composer::block_relay::v1alpha1::bid::Bundle,
};
use tokio::sync::{
    mpsc,
    oneshot,
};
use tonic::transport::Server;

pub(super) struct ProposerSubmission {
    // Address for the server to listen on
    addr: SocketAddr,
    // Channel to receive the next best builder bid for submission to proposer
    best_bid: mpsc::Sender<BidSender>,
    pending_commitment: Some(Bundle),
    // The proposer commitment currently waiting for finalization
    pending_payload: mpsc::Sender<GetTopOfBlockPayloadResponse>,
}

impl ProposerSubmission {
    pub(super) fn new(
        listening_addr: String,
        best_bid: mpsc::Sender<BidSender>,
        pending_payload: mpsc::Sender<GetTopOfBlockPayloadResponse>,
    ) -> eyre::Result<Self> {
        let addr = listening_addr
            .parse()
            .wrap_err("failed to parse proposer submission server listening addr")?;
        Ok(Self {
            addr,
            best_bid,
            pending_commitment: None,
            pending_payload,
        })
    }

    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        let Self {
            addr,
            best_bid,
            pending_commitment,
            pending_payload,
        } = &mut self;

        // create channel for server to request best_bid
        Server::builder()
            .add_service(TopOfBlockRelayServer::new(TopOfBlockRelayService::new()))
            .serve(addr)
            .await
            .wrap_err("failed to start proposer submission server")?;

        // select loop over pending commitment receiver and server crashing
        // if server crashes, return error
        // if pending commitment receiver returns, update state and print

        Ok(())
    }
}
