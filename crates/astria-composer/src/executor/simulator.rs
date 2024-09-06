use astria_core::execution::v1alpha2::{
    RollupData,
    RollupDataValue,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use bytes::Bytes;
use pbjson_types::Timestamp;
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
        let commitment_state = self
            .execution_service_client
            .get_commitment_state_with_retry()
            .await
            .wrap_err("failed to get commitment state")?;

        let soft_block = commitment_state.soft();

        // convert the sized bundle actions to a list of list of u8s
        // TODO - bharath - revisit this and make the code better. The else stmt is a bit weird
        let actions = bundle
            .into_actions()
            .iter()
            .map(|action| {
                return if let Some(seq_action) = action.as_sequence() {
                    seq_action.clone().data
                } else {
                    vec![]
                };
            })
            .filter(|data| !data.is_empty())
            .collect::<Vec<Vec<u8>>>();

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

        Ok(BundleSimulationResult::new(
            execute_block_response.included_transactions().to_vec(),
            soft_block.hash().clone(),
        ))
    }
}
