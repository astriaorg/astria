use std::sync::Arc;

use astria_core::{
    generated::{
        astria::sequencerblock::v1::{
            sequencer_service_server::SequencerService,
            FilteredSequencerBlock as RawFilteredSequencerBlock,
            GetFilteredSequencerBlockRequest,
            GetPendingNonceRequest,
            GetPendingNonceResponse,
            GetSequencerBlockRequest,
            SequencerBlock as RawSequencerBlock,
        },
        sequencerblock::v1::{
            GetUpgradesInfoRequest,
            GetUpgradesInfoResponse,
            GetValidatorNameRequest,
            GetValidatorNameResponse,
        },
    },
    primitive::v1::{
        Address,
        RollupId,
    },
    sequencerblock::v1::block::ExtendedCommitInfoWithProof,
    upgrades::v1::{
        Upgrade,
        Upgrades,
    },
    Protobuf,
};
use bytes::Bytes;
use cnidarium::Storage;
use tonic::{
    Request,
    Response,
    Status,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    app::StateReadExt as _,
    authority::StateReadExt as _,
    grpc::StateReadExt as _,
    mempool::Mempool,
};

pub(crate) struct SequencerServer {
    storage: Storage,
    mempool: Mempool,
    upgrades: Upgrades,
}

impl SequencerServer {
    pub(crate) fn new(storage: Storage, mempool: Mempool, upgrades: Upgrades) -> Self {
        Self {
            storage,
            mempool,
            upgrades,
        }
    }
}

#[async_trait::async_trait]
impl SequencerService for SequencerServer {
    /// Given a block height, returns the sequencer block at that height.
    #[instrument(skip_all)]
    async fn get_sequencer_block(
        self: Arc<Self>,
        request: Request<GetSequencerBlockRequest>,
    ) -> Result<Response<RawSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let block = snapshot
            .get_sequencer_block_by_height(request.height)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to get sequencer block from storage: {e}"))
            })?;

        Ok(Response::new(block.into_raw()))
    }

    /// Given a block height and set of rollup ids, returns a SequencerBlock which
    /// is filtered to contain only the transactions that are relevant to the given rollup.
    #[instrument(skip_all)]
    async fn get_filtered_sequencer_block(
        self: Arc<Self>,
        request: Request<GetFilteredSequencerBlockRequest>,
    ) -> Result<Response<RawFilteredSequencerBlock>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let curr_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e}"))
        })?;

        let request = request.into_inner();

        if curr_block_height < request.height {
            return Err(Status::invalid_argument(
                "requested height is greater than current block height",
            ));
        }

        let rollup_ids = request
            .rollup_ids
            .iter()
            .map(RollupId::try_from_raw_ref)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Status::invalid_argument(format!("invalid rollup ID: {e}")))?;

        let block_hash = snapshot
            .get_block_hash_by_height(request.height)
            .await
            .map_err(|e| Status::internal(format!("failed to get block hash from storage: {e}")))?;

        let header = snapshot
            .get_sequencer_block_header_by_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get sequencer block header from storage: {e}"
                ))
            })?;

        let rollup_transactions_proof = snapshot
            .get_rollup_transactions_proof_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get rollup transactions proof from storage: {e}"
                ))
            })?;

        let rollup_ids_proof = snapshot
            .get_rollup_ids_proof_by_block_hash(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!("failed to get rollup ids proof from storage: {e}"))
            })?;

        let upgrade_change_hashes = snapshot
            .get_upgrade_change_hashes(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get upgrade change hashes from storage: {e}"
                ))
            })?;

        let extended_commit_info_with_proof = snapshot
            .get_extended_commit_info_with_proof(&block_hash)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to get extended commit info with proof from storage: {e}"
                ))
            })?;

        let mut all_rollup_ids = snapshot
            .get_rollup_ids_by_block_hash(&block_hash)
            .await
            .map_err(|e| Status::internal(format!("failed to get rollup ids from storage: {e}")))?;
        all_rollup_ids.sort_unstable();

        // Filter out the Rollup Ids requested which have no data before grabbing
        // so as to not error because the block had no data for the requested rollup
        let rollup_ids: Vec<RollupId> = rollup_ids
            .into_iter()
            .filter(|id| all_rollup_ids.binary_search(id).is_ok())
            .collect();
        let mut rollup_transactions = Vec::with_capacity(rollup_ids.len());
        for rollup_id in rollup_ids {
            let rollup_data = snapshot
                .get_rollup_data(&block_hash, &rollup_id)
                .await
                .map_err(|e| {
                    Status::internal(format!("failed to get rollup data from storage: {e}",))
                })?;
            rollup_transactions.push(rollup_data.into_raw());
        }

        let all_rollup_ids = all_rollup_ids.into_iter().map(RollupId::into_raw).collect();

        let block = RawFilteredSequencerBlock {
            block_hash: Bytes::copy_from_slice(block_hash.as_bytes()),
            header: Some(header.into_raw()),
            rollup_transactions,
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
            all_rollup_ids,
            upgrade_change_hashes: upgrade_change_hashes
                .into_iter()
                .map(|change_hash| Bytes::copy_from_slice(change_hash.as_bytes()))
                .collect(),
            extended_commit_info_with_proof: extended_commit_info_with_proof
                .map(ExtendedCommitInfoWithProof::into_raw),
        };

        Ok(Response::new(block))
    }

    #[instrument(skip_all)]
    async fn get_pending_nonce(
        self: Arc<Self>,
        request: Request<GetPendingNonceRequest>,
    ) -> Result<Response<GetPendingNonceResponse>, Status> {
        use astria_core::primitive::v1::Address;

        use crate::accounts::StateReadExt as _;

        let request = request.into_inner();
        let Some(address) = request.address else {
            info!("required field address was not set",);
            return Err(Status::invalid_argument(
                "required field address was not set",
            ));
        };

        let address = Address::try_from_raw(address).map_err(|e| {
            info!(
                error = %e,
                "failed to parse address from request",
            );
            Status::invalid_argument(format!("invalid address: {e}"))
        })?;
        let nonce = self.mempool.pending_nonce(address.as_bytes()).await;

        if let Some(nonce) = nonce {
            return Ok(Response::new(GetPendingNonceResponse {
                inner: nonce,
            }));
        }

        // nonce wasn't in mempool, so just look it up from storage
        let snapshot = self.storage.latest_snapshot();
        let nonce = snapshot.get_account_nonce(&address).await.map_err(|e| {
            error!(
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                "failed to parse get account nonce from storage",
            );
            Status::internal(format!("failed to get account nonce from storage: {e}"))
        })?;

        Ok(Response::new(GetPendingNonceResponse {
            inner: nonce,
        }))
    }

    #[instrument(skip_all)]
    async fn get_upgrades_info(
        self: Arc<Self>,
        _request: Request<GetUpgradesInfoRequest>,
    ) -> Result<Response<GetUpgradesInfoResponse>, Status> {
        let snapshot = self.storage.latest_snapshot();
        let current_block_height = snapshot.get_block_height().await.map_err(|e| {
            Status::internal(format!("failed to get block height from storage: {e:#}"))
        })?;
        let mut response = GetUpgradesInfoResponse::default();
        // NOTE: the upgrades being added to the `response.applied` collection here have all been
        // verified as having been executed and added to verifiable storage during startup of the
        // sequencer. The ones added to `response.scheduled` can legitimately vary from sequencer to
        // sequencer depending upon what the operator has staged for upcoming upgrades. At the
        // activation point of scheduled upgrades, consensus must be reached on the contents of the
        // upgrade.  Assuming consensus is reached on the state changes caused by executing the
        // upgrade, any peer with varying upgrade details will produce a different state root hash
        // during finalize block at the activation height and will from there be stuck in a crash
        // loop.
        for change in self.upgrades.iter().flat_map(Upgrade::changes) {
            let change_info = change.info().to_raw();
            if change_info.activation_height <= current_block_height {
                response.applied.push(change_info);
            } else {
                response.scheduled.push(change_info);
            }
        }
        Ok(Response::new(response))
    }

    #[instrument(skip_all)]
    async fn get_validator_name(
        self: Arc<Self>,
        request: Request<GetValidatorNameRequest>,
    ) -> Result<Response<GetValidatorNameResponse>, Status> {
        let request = request.into_inner();
        let address = request
            .address
            .ok_or_else(|| Status::invalid_argument("required field address was not set"))?;
        let address = Address::try_from_raw(address).map_err(|e| {
            debug!(
                error = %e,
                "failed to parse address from get validator name request",
            );
            Status::invalid_argument(format!("invalid address: {e}"))
        })?;
        let snapshot = self.storage.latest_snapshot();
        let Some(validator) = snapshot.get_validator(&address).await.map_err(|e| {
            warn!(
                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                "failed to get validator from state",
            );
            Status::internal(format!("failed to get validator from state: {e}"))
        })?
        else {
            if snapshot.pre_aspen_get_validator_set().await.is_ok() {
                return Err(Status::failed_precondition(
                    "validator names are only supported post Aspen upgrade",
                ));
            }
            return Err(Status::not_found("provided address is not a validator"));
        };
        Ok(Response::new(GetValidatorNameResponse {
            name: validator.name.as_str().to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        protocol::{
            test_utils::ConfigureSequencerBlock,
            transaction::v1::action::ValidatorUpdate,
        },
        sequencerblock::v1::SequencerBlock,
    };
    use cnidarium::StateDelta;
    use telemetry::Metrics;

    use super::*;
    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
            },
            test_utils::get_alice_signing_key,
            StateWriteExt as _,
        },
        authority::{
            StateWriteExt as _,
            ValidatorSet,
        },
        benchmark_and_test_utils::{
            astria_address,
            verification_key,
        },
        grpc::StateWriteExt as _,
    };

    fn make_test_sequencer_block(height: u32) -> SequencerBlock {
        ConfigureSequencerBlock {
            height,
            ..Default::default()
        }
        .make()
    }

    #[tokio::test]
    async fn get_sequencer_block_ok() {
        let block = make_test_sequencer_block(1);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_tx = StateDelta::new(storage.latest_snapshot());
        state_tx.put_block_height(1).unwrap();
        state_tx.put_sequencer_block(block).unwrap();
        storage.commit(state_tx).await.unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetSequencerBlockRequest {
            height: 1,
        };
        let request = Request::new(request);
        let response = server.get_sequencer_block(request).await.unwrap();
        assert_eq!(response.into_inner().header.unwrap().height, 1);
    }

    #[tokio::test]
    async fn get_pending_nonce_in_mempool() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);

        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        // insert a transaction with a nonce gap
        let gapped_nonce = 99;
        let tx = crate::app::test_utils::MockTxBuilder::new()
            .nonce(gapped_nonce)
            .build();
        mempool
            .insert(tx, 0, &mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        // insert a transaction at the current nonce
        let account_nonce = 0;
        let tx = crate::app::test_utils::MockTxBuilder::new()
            .nonce(account_nonce)
            .build();

        mempool
            .insert(tx, 0, &mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        // insert a transactions one above account nonce (not gapped)
        let sequential_nonce = 1;
        let tx: Arc<astria_core::protocol::transaction::v1::Transaction> =
            crate::app::test_utils::MockTxBuilder::new()
                .nonce(sequential_nonce)
                .build();
        mempool
            .insert(tx, 0, &mock_balances(0, 0), mock_tx_cost(0, 0, 0))
            .await
            .unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, sequential_nonce + 1);
    }

    #[tokio::test]
    async fn get_pending_nonce_in_storage() {
        use crate::accounts::StateWriteExt as _;

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let mut state_tx = StateDelta::new(storage.latest_snapshot());
        let alice = get_alice_signing_key();
        let alice_address = astria_address(&alice.address_bytes());
        state_tx.put_account_nonce(&alice_address, 99).unwrap();
        storage.commit(state_tx).await.unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetPendingNonceRequest {
            address: Some(alice_address.into_raw()),
        };
        let request = Request::new(request);
        let response = server.get_pending_nonce(request).await.unwrap();
        assert_eq!(response.into_inner().inner, 99);
    }

    #[tokio::test]
    async fn get_validator_name_works_as_expected() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);
        let key_address_bytes = *verification_key.clone().address_bytes();
        let validator_name = "test".to_string();

        let update_with_name = ValidatorUpdate {
            name: validator_name.clone().parse().unwrap(),
            power: 100,
            verification_key,
        };

        state.put_validator(&update_with_name).unwrap();
        storage.commit(state).await.unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetValidatorNameRequest {
            address: Some(astria_address(&key_address_bytes).into_raw()),
        };
        let rsp = server
            .get_validator_name(Request::new(request))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rsp.name, validator_name);
    }

    #[tokio::test]
    async fn validator_name_request_fails_if_not_a_validator() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let storage = cnidarium::TempStorage::new().await.unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetValidatorNameRequest {
            address: Some(astria_address(&[0; 20]).into_raw()),
        };

        let rsp = server
            .get_validator_name(Request::new(request))
            .await
            .unwrap_err();
        assert_eq!(rsp.code(), tonic::Code::NotFound, "{}", rsp.message());
        let err_msg = "provided address is not a validator";
        assert_eq!(rsp.message(), err_msg);
    }

    #[tokio::test]
    async fn validator_name_request_fails_if_pre_aspen() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);
        let key_address_bytes = *verification_key.clone().address_bytes();
        let validator_name = "test".to_string();

        let validator_set = ValidatorSet::new_from_updates(vec![ValidatorUpdate {
            name: validator_name.clone().parse().unwrap(),
            power: 100,
            verification_key,
        }]);

        state.pre_aspen_put_validator_set(validator_set).unwrap();
        storage.commit(state).await.unwrap();

        let server = Arc::new(SequencerServer::new(
            storage.clone(),
            mempool,
            Upgrades::default(),
        ));
        let request = GetValidatorNameRequest {
            address: Some(astria_address(&key_address_bytes).into_raw()),
        };
        let rsp = server
            .get_validator_name(Request::new(request))
            .await
            .unwrap_err();
        assert_eq!(
            rsp.code(),
            tonic::Code::FailedPrecondition,
            "{}",
            rsp.message()
        );
        let err_msg = "validator names are only supported post Aspen upgrade";
        assert_eq!(rsp.message(), err_msg);
    }
}
