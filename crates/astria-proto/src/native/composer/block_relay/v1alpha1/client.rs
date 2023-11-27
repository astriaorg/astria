use ed25519_consensus::SigningKey;
use tonic::{
    async_trait,
    transport::Channel,
};

use super::bid::{
    self,
    Bundle,
    OpaqueBid,
    SignedBundle,
};
use crate::generated::block_relay::v1alpha1::{
    top_of_block_relay_client::TopOfBlockRelayClient,
    GetBidRequest,
    GetBundleRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("client failed to deliver message to relay")]
    RequestFailed(#[source] tonic::Status),
    #[error("failed to decode bid")]
    BidDecodeFailed(#[source] bid::OpaqueBidError),
    #[error("commitment does not match bid")]
    InvalidCommitment,
}

// TODO: use bid object instead of request object and map to error enum from response
#[async_trait]
/// Client for interacting with the block relay service. This client is used by the Proposer in
/// PrepareProposal to get the top of block bid and payload from the relay.
pub trait Client {
    /// Get the top of block bid from the relay
    async fn get_bid(&mut self, block_height: u64) -> Result<OpaqueBid, Error>;

    /// Get the payload for the given bid. Constructs the commitment to the bid using the provided
    /// signer.
    async fn get_bundle(
        &mut self,
        block_height: u64,
        bid: OpaqueBid,
        signer: SigningKey,
    ) -> Result<Bundle, Error>;
}

#[async_trait]
impl Client for TopOfBlockRelayClient<Channel> {
    /// Get the top of block bid from the relay.
    async fn get_bid(&mut self, block_height: u64) -> Result<OpaqueBid, Error> {
        let request = GetBidRequest {
            block_height,
        };
        let response = self
            .get_bid(request)
            .await
            .map_err(Error::RequestFailed)?
            .into_inner();

        todo!("why doesn't this compile?");
        let raw_bid = response.bid.ok_or(Error::BidDecodeFailed)?;
        let bid = OpaqueBid::try_from_raw(raw_bid)?;
        Ok(bid)
    }

    /// Get the payload for the given bid. Constructs the commitment to the bid using the provided
    /// signer.
    async fn get_bundle(
        &mut self,
        block_height: u64,
        bid: OpaqueBid,
        signer: SigningKey,
    ) -> Result<Bundle, Error> {
        let commitment = bid.commitment(signer);

        let request = GetBundleRequest {
            bid: Some(bid.into_raw()),
            commitment: commitment.to_vec(),
        };
        let response = self
            .get_bundle(request)
            .await
            .map_err(Error::RequestFailed)?
            .into_inner();

        todo!("why doesn't this compile?");
        let raw_bundle = response.bundle.ok_or(Error::RequestFailed)?;
        let signed_bundle = SignedBundle::try_from_raw(raw_bundle)?;
        Ok(signed_bundle.bundle)
    }
}
