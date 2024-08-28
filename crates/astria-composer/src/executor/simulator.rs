use astria_core::execution::v1alpha2::Block;
/// ! `BundleSimulator` is responsible for fetching the latest rollup commitment state
/// and simulating the given bundle on top of the latest soft block.
use astria_core::{
    sequencerblock::v1alpha1::block::RollupData,
    Protobuf,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use bytes::Bytes;
use pbjson_types::Timestamp;
use prost::Message;
use tracing::{
    info,
    instrument,
};

use crate::executor::{
    bundle_factory::SizedBundle,
    client::Client,
};

#[derive(Clone)]
pub(crate) struct BundleSimulator {
    execution_service_client: Client,
}

pub(crate) struct BundleSimulationResult {
    block: Block,
    included_actions: Vec<RollupData>,
    parent_hash: Bytes,
}

impl BundleSimulationResult {
    pub(crate) fn new(
        included_sequence_actions: Vec<RollupData>,
        block: Block,
        parent_hash: Bytes,
    ) -> Self {
        Self {
            block,
            included_actions: included_sequence_actions,
            parent_hash,
        }
    }

    pub(crate) fn included_actions(&self) -> &[RollupData] {
        self.included_actions.as_slice()
    }

    pub(crate) fn parent_hash(&self) -> Bytes {
        self.parent_hash.clone()
    }

    pub(crate) fn block(&self) -> &Block {
        &self.block
    }
}

impl BundleSimulator {
    pub(crate) fn new(execution_api_uri: &str) -> eyre::Result<Self> {
        Ok(Self {
            execution_service_client: Client::connect_lazy(execution_api_uri)
                .wrap_err("failed to connect to execution service")?,
        })
    }

    // TODO - the interfaces below are weird but they work for now
    // have cleaner interfaces
    #[instrument(skip_all, fields(uri=self.execution_service_client.uri()))]
    pub(crate) async fn simulate_parent_bundle(
        self,
        rollup_data: Vec<RollupData>,
        time: pbjson_types::Timestamp,
    ) -> eyre::Result<BundleSimulationResult> {
        // call GetCommitmentState to get the soft block
        let commitment_state = self
            .execution_service_client
            .get_commitment_state_with_retry()
            .await
            .wrap_err("failed to get commitment state")?;

        let soft_block = commitment_state.soft();
        // convert the sized bundle actions to a list of Vec<u8>
        let actions: Vec<Vec<u8>> = rollup_data
            .iter()
            .map(|action| match action.clone() {
                RollupData::SequencedData(data) => data.to_vec(),
                _ => vec![],
            })
            .filter(|data| !data.is_empty())
            .collect();

        // as long as the timestamp > parent block timestamp, the block will be successfully
        // created. It doesn't matter what timestamp we use anyway since we are not going to
        // commit the block to the chain.
        // call execute block with the bundle to get back the included transactions
        let execute_block_response = self
            .execution_service_client
            .execute_block_with_retry(
                soft_block.hash().clone(),
                actions,
                // use current timestamp
                time,
                false,
            )
            .await
            .wrap_err("failed to execute block")?;

        let included_transactions = execute_block_response.included_transactions();
        info!(
            "Parent block created on top of {:?} and {:?} transactions were included",
            soft_block.hash(),
            included_transactions.len()
        );
        Ok(BundleSimulationResult::new(
            included_transactions.to_vec(),
            execute_block_response.block().clone(),
            soft_block.hash().clone(),
        ))
    }

    #[instrument(skip_all, fields(uri=self.execution_service_client.uri()), err)]
    pub(crate) async fn simulate_bundle_on_block(
        self,
        bundle: SizedBundle,
        block: Block,
    ) -> eyre::Result<BundleSimulationResult> {
        // convert the sized bundle actions to a list of Vec<u8>
        let actions: Vec<Vec<u8>> = bundle
            .into_actions()
            .iter()
            .map(|action| match action.as_sequence() {
                Some(seq_action) => RollupData::SequencedData(seq_action.clone().data)
                    .to_raw()
                    .encode_to_vec(),
                None => vec![],
            })
            .filter(|data| !data.is_empty())
            .collect();

        // as long as the timestamp > parent block timestamp, the block will be successfully
        // created. It doesn't matter what timestamp we use anyway since we are not going to
        // commit the block to the chain.
        let timestamp = Timestamp {
            seconds: block.timestamp().seconds + 3,
            nanos: 0,
        };
        // call execute block with the bundle to get back the included transactions
        let execute_block_response = self
            .execution_service_client
            .execute_block_with_retry(
                block.hash().clone(),
                actions,
                // use current timestamp
                timestamp,
                true,
            )
            .await
            .wrap_err("failed to execute block")?;

        let included_transactions = execute_block_response.included_transactions();
        info!(
            "Bundle simulated on top of {:?} and {:?} transactions were included",
            block.hash().clone(),
            included_transactions.len()
        );
        Ok(BundleSimulationResult::new(
            included_transactions.to_vec(),
            execute_block_response.block().clone(),
            block.hash().clone(),
        ))
    }

    #[instrument(skip_all, fields(uri=self.execution_service_client.uri()))]
    pub(crate) async fn simulate_bundle(
        self,
        bundle: SizedBundle,
    ) -> eyre::Result<BundleSimulationResult> {
        // call GetCommitmentState to get the soft block
        info!("Calling GetCommitmentState!");
        let commitment_state = self
            .execution_service_client
            .get_commitment_state_with_retry()
            .await
            .wrap_err("failed to get commitment state")?;
        info!("Received CommitmentState of rollup");

        let soft_block = commitment_state.soft();
        info!("Soft block hash is {:?}", soft_block.hash());
        // convert the sized bundle actions to a list of Vec<u8>
        let actions: Vec<Vec<u8>> = bundle
            .into_actions()
            .iter()
            .map(|action| match action.as_sequence() {
                Some(seq_action) => RollupData::SequencedData(seq_action.clone().data)
                    .to_raw()
                    .encode_to_vec(),
                None => vec![],
            })
            .filter(|data| !data.is_empty())
            .collect();

        info!("Calling ExecuteBlock to simulate the bundle!");
        // as long as the timestamp > parent block timestamp, the block will be successfully
        // created. It doesn't matter what timestamp we use anyway since we are not going to
        // commit the block to the chain.
        let timestamp = Timestamp {
            seconds: soft_block.timestamp().seconds + 3,
            nanos: 0,
        };
        // call execute block with the bundle to get back the included transactions
        let execute_block_response = self
            .execution_service_client
            .execute_block_with_retry(
                soft_block.hash().clone(),
                actions,
                // use current timestamp
                timestamp,
                true,
            )
            .await
            .wrap_err("failed to execute block")?;

        let included_transactions = execute_block_response.included_transactions();
        info!(
            "Bundle simulated on top of {:?} and {:?} transactions were included",
            soft_block.hash(),
            included_transactions.len()
        );
        Ok(BundleSimulationResult::new(
            included_transactions.to_vec(),
            execute_block_response.block().clone(),
            soft_block.hash().clone(),
        ))
    }
}
