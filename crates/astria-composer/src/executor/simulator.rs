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
use tracing::instrument;

use crate::executor::{
    bundle_factory::SizedBundle,
    client::Client,
};

#[derive(Clone)]
pub(crate) struct BundleSimulator {
    execution_service_client: Client,
}

pub(crate) struct BundleSimulationResult {
    included_actions: Vec<RollupData>,
    parent_hash: Bytes,
}

impl BundleSimulationResult {
    pub(crate) fn new(included_sequence_actions: Vec<RollupData>, parent_hash: Bytes) -> Self {
        Self {
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
}

impl BundleSimulator {
    pub(crate) fn new(execution_api_uri: &str) -> eyre::Result<Self> {
        Ok(Self {
            execution_service_client: Client::connect_lazy(execution_api_uri)
                .wrap_err("failed to connect to execution service")?,
        })
    }

    #[instrument(skip_all, fields(uri=self.execution_service_client.uri()))]
    pub(crate) async fn simulate_bundle(
        self,
        bundle: SizedBundle,
    ) -> eyre::Result<BundleSimulationResult> {
        // call GetCommitmentState to get the soft block
        println!("IN MAIN CODE: CALLING GET COMMITMENT STATE");
        let commitment_state = self
            .execution_service_client
            .get_commitment_state_with_retry()
            .await
            .wrap_err("failed to get commitment state")?;

        println!("IN MAIN CODE: CALLED GET COMMITMENT STATE!");
        let soft_block = commitment_state.soft();
        // convert the sized bundle actions to a list of list of u8s
        // TODO - bharath - revisit this and make the code better. The else stmt is a bit weird
        let actions: Vec<Vec<u8>> = bundle
            .into_actions()
            .iter()
            .map(|action| {
                // TODO - should we support sequencer transfers and actions outside sequence
                // actions too?
                return if let Some(seq_action) = action.as_sequence() {
                    RollupData::SequencedData(seq_action.clone().data)
                        .to_raw()
                        .encode_to_vec()
                } else {
                    vec![]
                };
            })
            .filter(|data| !data.is_empty())
            .collect();

        println!("IN MAIN CODE: CALLING EXECUTE_BLOCK");
        // call execute block with the bundle to get back the included transactions
        let execute_block_response = self
            .execution_service_client
            .execute_block_with_retry(
                soft_block.hash().clone(),
                actions,
                Timestamp::from(soft_block.timestamp()),
                true,
            )
            .await
            .wrap_err("failed to execute block")?;

        println!("IN MAIN CODE: CALLED EXECUTE BLOCK!!");
        Ok(BundleSimulationResult::new(
            execute_block_response.included_transactions().to_vec(),
            execute_block_response.block().parent_block_hash().clone(),
        ))
    }
}
