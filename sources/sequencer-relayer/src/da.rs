use anyhow::{anyhow, Error};
use async_trait::async_trait;
use rs_cnc::{CelestiaNodeClient, NamespacedSharesResponse, PayForDataResponse};
use serde::{Deserialize, Serialize};

use crate::types::{Base64String, Block};

static DEFAULT_PFD_FEE: i64 = 2_000;
static DEFAULT_PFD_GAS_LIMIT: u64 = 90_000;
static DEFAULT_NAMESPACE: &str = "0011223344556677"; // TODO; hash something to get this

/// SequencerBlock represents a sequencer layer block to be submitted to
/// the DA layer.
/// Currently, it consists of the Block.Data field of the cosmos-sdk block
/// returned by a sequencer, which contains the block's transactions.
/// TODO: include other fields (such as block hash)?
/// TODO: compression or a better serialization method?
/// TODO: rename this b/c it's kind of confusing, types::Block is a cosmos-sdk block
/// which is also a sequencer block in a way.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SequencerBlock {
    txs: Vec<Base64String>,
}

impl SequencerBlock {
    pub fn new() -> Self {
        SequencerBlock { txs: vec![] }
    }
}

impl From<Block> for SequencerBlock {
    fn from(b: Block) -> Self {
        // TODO: we need to unwrap sequencer txs into rollup-specific txs here,
        // and namespace them correspondingly
        Self { txs: b.data.txs }
    }
}

#[derive(Deserialize, Debug)]
pub struct SubmitBlockResponse(pub PayForDataResponse);

#[derive(Deserialize, Debug)]
pub struct CheckBlockAvailabilityResponse(pub NamespacedSharesResponse);

/// DataAvailabilityClient is able to submit and query blocks from the DA layer.
#[async_trait]
pub trait DataAvailabilityClient {
    async fn submit_block(&self, block: SequencerBlock) -> Result<SubmitBlockResponse, Error>;
    async fn check_block_availability(
        &self,
        height: u64,
    ) -> Result<CheckBlockAvailabilityResponse, Error>;
    async fn get_blocks(&self, height: u64) -> Result<Vec<SequencerBlock>, Error>;
}

pub struct CelestiaClient(CelestiaNodeClient);

impl CelestiaClient {
    pub fn new(endpoint: String) -> Result<Self, Error> {
        let cnc = CelestiaNodeClient::new(endpoint)?;
        Ok(CelestiaClient(cnc))
    }
}

#[async_trait]
impl DataAvailabilityClient for CelestiaClient {
    async fn submit_block(&self, block: SequencerBlock) -> Result<SubmitBlockResponse, Error> {
        // TODO: don't use json, use our own serializer
        let block_bytes = serde_json::to_string(&block).map_err(|e| anyhow!(e))?;
        let pay_for_data_response = self
            .0
            .submit_pay_for_data(
                DEFAULT_NAMESPACE,
                &block_bytes.into(),
                DEFAULT_PFD_FEE,
                DEFAULT_PFD_GAS_LIMIT,
            )
            .await?;
        Ok(SubmitBlockResponse(pay_for_data_response))
    }

    async fn check_block_availability(
        &self,
        height: u64,
    ) -> Result<CheckBlockAvailabilityResponse, Error> {
        let resp = self.0.namespaced_shares(DEFAULT_NAMESPACE, height).await?;
        Ok(CheckBlockAvailabilityResponse(resp))
    }

    async fn get_blocks(&self, height: u64) -> Result<Vec<SequencerBlock>, Error> {
        let namespaced_data_response = self.0.namespaced_data(DEFAULT_NAMESPACE, height).await?;

        let blocks = namespaced_data_response
            .data
            .unwrap_or_default()
            .iter()
            .filter_map(|d| {
                let block: Option<SequencerBlock> = serde_json::from_slice(&d.0).ok();
                block
            })
            .collect();
        Ok(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::{CelestiaClient, DataAvailabilityClient, SequencerBlock};
    use crate::types::Base64String;

    #[tokio::test]
    async fn test_celestia_client() {
        // unfortunately, this needs to be all one test for now, since
        // submitting multiple blocks to celestia concurrently returns
        // "incorrect account sequence" errors.

        // test submit_block
        let base_url = "http://localhost:26659".to_string();
        let client = CelestiaClient::new(base_url).unwrap();
        let tx = Base64String(b"noot_was_here".to_vec());
        let block = SequencerBlock {
            txs: vec![tx.clone()],
        };
        let submit_block_resp = client.submit_block(block).await.unwrap();

        // test check_block_availability
        let resp = client
            .check_block_availability(submit_block_resp.0.height.unwrap())
            .await
            .unwrap();
        assert_eq!(resp.0.height, submit_block_resp.0.height.unwrap());

        // test get_blocks
        let resp = client
            .get_blocks(submit_block_resp.0.height.unwrap())
            .await
            .unwrap();
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].txs.len(), 1);
        assert_eq!(resp[0].txs[0], tx);
    }
}
