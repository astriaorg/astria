use ed25519_consensus::SigningKey;
use proto::generated::block_relay::v1alpha1::{
    top_of_block_relay_client::TopOfBlockRelayClient,
    GetTopOfBlockBidRequest,
    GetTopOfBlockBidResponse,
    GetTopOfBlockPayloadRequest,
    GetTopOfBlockPayloadResponse,
};
use tonic::transport::Channel;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to connect to relay")]
    ConnectionFailed(#[source] tonic::transport::Error),
    #[error("client failed to deliver message to relay")]
    RequestFailed(#[source] tonic::Status),
    #[error("cannot get payload without committing to bid")]
    NoBidExists,
}

pub struct Client {
    // Client for sending messages to the block relay service.
    relay_client: TopOfBlockRelayClient<Channel>,
    // The latest bid received from the relay
    bid: Option<GetTopOfBlockBidResponse>,
    // The proposer's key for signing commitments
    proposer_key: SigningKey,
}

impl Client {
    // Create a new TopOfBlockClient with no bid intialized
    pub async fn new(relay_address: String, proposer_key: SigningKey) -> Result<Self, Error> {
        let relay_client = Client::connect(relay_address)
            .await
            .map_err(Error::ConnectionFailed)?;
        Ok(Self {
            relay_client,
            bid: None,
            proposer_key,
        })
    }

    // Get the top of block bid from the relay
    pub async fn get_bid(&mut self, block_height: u64) -> Result<GetTopOfBlockBidResponse, Error> {
        let request =
            GetTopOfBlockBidRequest {
                block_height,
            };
        let response = self
            .relay_client
            .get_top_of_block_bid(request)
            .await
            .map_err(Error::RequestFailed)?
            .into_inner();
        Ok(response)
    }

    pub async fn get_payload(
        &mut self,
        block_height: u64,
    ) -> Result<GetTopOfBlockPayloadResponse, Error> {
        let builder_address = self
            .bid
            .as_ref()
            .ok_or(Error::NoBidExists)?
            .builder_address
            .clone();

        let payload_hash = self
            .bid
            .as_ref()
            .ok_or(Error::NoBidExists)?
            .payload_hash
            .clone();

        // TODO: commitment should be a byte slice with defined length
        let commitment = self.proposer_key.sign(&payload_hash).to_bytes().to_vec();

        let request =
            GetTopOfBlockPayloadRequest {
                builder_address,
                block_height,
                commitment,
            };
        let response = self
            .relay_client
            .get_top_of_block_payload(request)
            .await
            .map_err(Error::RequestFailed)?
            .into_inner();
        Ok(response)
    }
}
