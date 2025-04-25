use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    bail,
    Result,
    WrapErr as _,
};
use cnidarium::Storage;
use tendermint::v0_38::abci::{
    request,
    response,
    ConsensusRequest,
    ConsensusResponse,
};
use tokio::sync::mpsc;
use tower_abci::BoxError;
use tower_actor::Message;
use tracing::{
    debug,
    info,
    instrument,
    warn,
    Instrument as _,
    Level,
};

use crate::app::{
    App,
    ShouldShutDown,
};

pub(crate) struct Consensus {
    queue: mpsc::Receiver<Message<ConsensusRequest, ConsensusResponse, tower::BoxError>>,
    storage: Storage,
    app: App,
    cancellation_token: tokio_util::sync::CancellationToken,
}

impl Consensus {
    pub(crate) fn new(
        storage: Storage,
        app: App,
        queue: mpsc::Receiver<Message<ConsensusRequest, ConsensusResponse, tower::BoxError>>,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            queue,
            storage,
            app,
            cancellation_token,
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), tower::BoxError> {
        while let Some(Message {
            req,
            rsp_sender,
            span,
        }) = self.queue.recv().await
        {
            // The send only fails if the receiver was dropped, which happens
            // if the caller didn't propagate the message back to tendermint
            // for some reason -- but that's not our problem.
            let (rsp, should_shut_down) =
                match self.handle_request(req).instrument(span.clone()).await {
                    Ok(ok_res) => ok_res,
                    Err(e) => {
                        panic!("failed to handle consensus request, this is a bug: {e:?}");
                    }
                };
            // `send` returns the sent message if sending fail, so we are dropping it.
            if rsp_sender.send(Ok(rsp)).is_err() {
                warn!(
                    parent: &span,
                    "failed returning consensus response to request sender; dropping response"
                );
            }
            if let ShouldShutDown::ShutDownForUpgrade {
                upgrade_activation_height,
                block_time,
                hex_encoded_app_hash,
            } = should_shut_down
            {
                info!(
                    upgrade_activation_height,
                    latest_app_hash = %hex_encoded_app_hash,
                    latest_block_time = %block_time,
                    "shutting down for upgrade"
                );
                self.cancellation_token.cancel();
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_request(
        &mut self,
        req: ConsensusRequest,
    ) -> Result<(ConsensusResponse, ShouldShutDown), BoxError> {
        Ok(match req {
            ConsensusRequest::InitChain(init_chain) => (
                ConsensusResponse::InitChain(
                    self.init_chain(init_chain)
                        .await
                        .wrap_err("failed initializing chain")?,
                ),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::PrepareProposal(prepare_proposal) => (
                ConsensusResponse::PrepareProposal(
                    self.handle_prepare_proposal(prepare_proposal)
                        .await
                        .wrap_err("failed to prepare proposal")?,
                ),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::ProcessProposal(process_proposal) => (
                ConsensusResponse::ProcessProposal(
                    match self.handle_process_proposal(process_proposal).await {
                        Ok(()) => response::ProcessProposal::Accept,
                        Err(e) => {
                            warn!(
                                error = AsRef::<dyn std::error::Error>::as_ref(&e),
                                "rejecting proposal"
                            );
                            response::ProcessProposal::Reject
                        }
                    },
                ),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::ExtendVote(extend_vote) => (
                ConsensusResponse::ExtendVote(match self.handle_extend_vote(extend_vote).await {
                    Ok(response) => response,
                    Err(e) => {
                        warn!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&e),
                            "failed to extend vote, returning empty vote extension"
                        );
                        response::ExtendVote {
                            vote_extension: vec![].into(),
                        }
                    }
                }),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::VerifyVoteExtension(vote_extension) => (
                ConsensusResponse::VerifyVoteExtension(
                    self.handle_verify_vote_extension(vote_extension)
                        .await
                        .wrap_err("failed to verify vote extension")?,
                ),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::FinalizeBlock(finalize_block) => (
                ConsensusResponse::FinalizeBlock(
                    self.finalize_block(finalize_block)
                        .await
                        .wrap_err("failed to finalize block")?,
                ),
                ShouldShutDown::ContinueRunning,
            ),
            ConsensusRequest::Commit => {
                let (rsp, should_shut_down) = self.commit().await.wrap_err("failed to commit")?;
                (ConsensusResponse::Commit(rsp), should_shut_down)
            }
        })
    }

    #[instrument(skip_all, err)]
    async fn init_chain(&mut self, init_chain: request::InitChain) -> Result<response::InitChain> {
        // the storage version is set to u64::MAX by default when first created
        if self.storage.latest_version() != u64::MAX {
            bail!("database already initialized");
        }

        let genesis_state: GenesisAppState = serde_json::from_slice(&init_chain.app_state_bytes)
            .wrap_err("failed to parse app_state in genesis file")?;
        let app_hash = self
            .app
            .init_chain(
                self.storage.clone(),
                genesis_state,
                init_chain
                    .validators
                    .iter()
                    .cloned()
                    .map(crate::utils::cometbft_to_sequencer_validator)
                    .collect::<Result<_, _>>()
                    .wrap_err(
                        "failed converting cometbft genesis validators to astria validators",
                    )?,
                init_chain.chain_id,
            )
            .await
            .wrap_err("failed to call init_chain")?;
        self.app
            .commit(self.storage.clone())
            .await
            .wrap_err("failed to commit")?;

        Ok(response::InitChain {
            app_hash,
            consensus_params: Some(init_chain.consensus_params),
            validators: init_chain.validators,
        })
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn handle_prepare_proposal(
        &mut self,
        prepare_proposal: request::PrepareProposal,
    ) -> Result<response::PrepareProposal> {
        self.app
            .prepare_proposal(prepare_proposal, self.storage.clone())
            .await
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn handle_process_proposal(
        &mut self,
        process_proposal: request::ProcessProposal,
    ) -> Result<()> {
        self.app
            .process_proposal(process_proposal, self.storage.clone())
            .await?;
        debug!("proposal processed");
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn handle_extend_vote(
        &mut self,
        extend_vote: request::ExtendVote,
    ) -> Result<response::ExtendVote> {
        let extend_vote = self.app.extend_vote(extend_vote).await?;
        Ok(extend_vote)
    }

    #[instrument(skip_all, err(level = Level::WARN))]
    async fn handle_verify_vote_extension(
        &mut self,
        vote_extension: request::VerifyVoteExtension,
    ) -> Result<response::VerifyVoteExtension> {
        self.app.verify_vote_extension(vote_extension).await
    }

    #[instrument(
        skip_all,
        fields(
            hash = %finalize_block.hash,
            height = %finalize_block.height,
            time = %finalize_block.time,
            proposer = %finalize_block.proposer_address
        ),
        err
    )]
    async fn finalize_block(
        &mut self,
        finalize_block: request::FinalizeBlock,
    ) -> Result<response::FinalizeBlock> {
        let finalize_block = self
            .app
            .finalize_block(finalize_block, self.storage.clone())
            .await
            .wrap_err("failed to call App::finalize_block")?;
        Ok(finalize_block)
    }

    #[instrument(skip_all)]
    async fn commit(&mut self) -> Result<(response::Commit, ShouldShutDown)> {
        let should_shut_down = self
            .app
            .commit(self.storage.clone())
            .await
            .wrap_err("error committing")?;
        Ok((response::Commit::default(), should_shut_down))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        str::FromStr,
        sync::Arc,
    };

    use astria_core::{
        crypto::{
            SigningKey,
            VerificationKey,
        },
        generated::astria::protocol::genesis::v1::{
            Account as RawAccount,
            GenesisAppState as RawGenesisAppState,
        },
        primitive::v1::RollupId,
        protocol::transaction::v1::{
            action::RollupDataSubmission,
            Transaction,
            TransactionBody,
        },
        sequencerblock::v1::DataItem,
        Protobuf as _,
    };
    use bytes::Bytes;
    use rand::rngs::OsRng;
    use tendermint::{
        abci::types::{
            CommitInfo,
            ExtendedCommitInfo,
        },
        account::Id,
        Hash,
        Time,
    };

    use super::*;
    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
                proto_genesis_state,
                AppInitializer,
            },
            test_utils::{
                run_until_aspen_applied,
                transactions_with_extended_commit_info_and_commitments,
            },
        },
        benchmark_and_test_utils::astria_address,
        mempool::Mempool,
    };

    const BLOCK_HEIGHT: u8 = 100;

    fn make_unsigned_tx() -> TransactionBody {
        TransactionBody::builder()
            .actions(vec![RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                data: Bytes::from_static(b"hello world"),
                fee_asset: crate::benchmark_and_test_utils::nria().into(),
            }
            .into()])
            .chain_id("test")
            .try_build()
            .unwrap()
    }

    fn new_prepare_proposal_request() -> request::PrepareProposal {
        request::PrepareProposal {
            txs: vec![],
            max_tx_bytes: 1024,
            local_last_commit: Some(ExtendedCommitInfo {
                round: 0u16.into(),
                votes: vec![],
            }),
            misbehavior: vec![],
            height: BLOCK_HEIGHT.into(),
            time: Time::now(),
            next_validators_hash: Hash::default(),
            proposer_address: Id::from_str("0CDA3F47EF3C4906693B170EF650EB968C5F4B2C").unwrap(),
        }
    }

    fn new_process_proposal_request(txs: &[Arc<Transaction>]) -> request::ProcessProposal {
        let height = tendermint::block::Height::from(BLOCK_HEIGHT);
        request::ProcessProposal {
            txs: transactions_with_extended_commit_info_and_commitments(height, txs, None),
            proposed_last_commit: Some(CommitInfo {
                round: 0u16.into(),
                votes: vec![],
            }),
            misbehavior: vec![],
            hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
            height,
            next_validators_hash: Hash::default(),
            time: Time::now(),
            proposer_address: Id::from_str("0CDA3F47EF3C4906693B170EF650EB968C5F4B2C").unwrap(),
        }
    }

    async fn new_consensus_service(funded_key: Option<VerificationKey>) -> (Consensus, Mempool) {
        let accounts = funded_key
            .into_iter()
            .map(|funded_key| RawAccount {
                address: Some(astria_address(funded_key.address_bytes()).to_raw()),
                balance: Some(10u128.pow(19).into()),
            })
            .collect();
        let genesis_state = RawGenesisAppState {
            accounts,
            ..proto_genesis_state()
        }
        .try_into()
        .unwrap();

        let (mut app, storage) = AppInitializer::new()
            .with_genesis_state(genesis_state)
            .init()
            .await;
        let _ = run_until_aspen_applied(&mut app, storage.clone()).await;

        let (_tx, rx) = mpsc::channel(1);
        let cancellation_token = tokio_util::sync::CancellationToken::new();
        let mempool = app.mempool();
        let consensus = Consensus::new(storage.clone(), app, rx, cancellation_token);
        (consensus, mempool)
    }

    #[tokio::test]
    async fn prepare_and_process_proposal() {
        let signing_key = SigningKey::new(OsRng);
        let (mut consensus_service, mempool) =
            new_consensus_service(Some(signing_key.verification_key())).await;
        let tx = make_unsigned_tx();
        let signed_tx = Arc::new(tx.sign(&signing_key));
        mempool
            .insert(
                signed_tx.clone(),
                0,
                mock_balances(0, 0),
                mock_tx_cost(0, 0, 0),
            )
            .await
            .unwrap();

        let prepare_proposal = new_prepare_proposal_request();
        let prepare_proposal_response = consensus_service
            .handle_prepare_proposal(prepare_proposal)
            .await
            .unwrap();

        let process_proposal = new_process_proposal_request(&[signed_tx.clone()]);
        let expected_txs: Vec<Bytes> = process_proposal.txs.clone();

        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: expected_txs,
            }
        );

        let (mut consensus_service, _) =
            new_consensus_service(Some(signing_key.verification_key())).await;
        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn process_proposal_ok() {
        let signing_key = SigningKey::new(OsRng);
        let (mut consensus_service, _) =
            new_consensus_service(Some(signing_key.verification_key())).await;
        let tx = make_unsigned_tx();
        let signed_tx = Arc::new(tx.sign(&signing_key));
        let process_proposal = new_process_proposal_request(&[signed_tx]);

        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn process_proposal_fail_missing_action_commitment() {
        let (mut consensus_service, _) = new_consensus_service(None).await;
        let mut process_proposal = new_process_proposal_request(&[]);
        process_proposal.txs.clear();
        let error_message = format!(
            "{:#}",
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
        );
        let expected = "did not contain the rollup transactions root";
        assert!(
            error_message.contains(expected),
            "`{error_message}` didn't contain `{expected}`"
        );
    }

    #[tokio::test]
    async fn process_proposal_fail_wrong_commitment_length() {
        let (mut consensus_service, _) = new_consensus_service(None).await;
        let mut process_proposal = new_process_proposal_request(&[]);
        process_proposal.txs = vec![[0u8; 16].to_vec().into()];
        let error_message = format!(
            "{:#}",
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
        );
        let expected = "item 0 of cometbft `block.data` could not be protobuf-decoded";
        assert!(
            error_message.contains(expected),
            "`{error_message}` didn't contain `{expected}`"
        );
    }

    #[tokio::test]
    async fn process_proposal_fail_wrong_commitment_value() {
        let (mut consensus_service, _) = new_consensus_service(None).await;
        let mut process_proposal = new_process_proposal_request(&[]);
        process_proposal.txs[0] = DataItem::RollupTransactionsRoot([99u8; 32]).encode();
        let error_message = format!(
            "{:#}",
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
        );
        let expected = "rollup transactions commitment does not match expected";
        assert!(
            error_message.contains(expected),
            "`{error_message}` didn't contain `{expected}`"
        );
    }

    #[tokio::test]
    async fn prepare_proposal_empty_block() {
        let (mut consensus_service, _) = new_consensus_service(None).await;
        let prepare_proposal = new_prepare_proposal_request();

        let prepare_proposal_response = consensus_service
            .handle_prepare_proposal(prepare_proposal)
            .await
            .unwrap();

        let expected_txs =
            transactions_with_extended_commit_info_and_commitments(BLOCK_HEIGHT.into(), &[], None);
        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: expected_txs,
            }
        );
    }

    #[tokio::test]
    async fn process_proposal_ok_empty_block() {
        let (mut consensus_service, _) = new_consensus_service(None).await;
        let process_proposal = new_process_proposal_request(&[]);
        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn block_lifecycle() {
        let signing_key = SigningKey::new(OsRng);
        let address_bytes = *signing_key.verification_key().address_bytes();
        let (mut consensus_service, mempool) =
            new_consensus_service(Some(signing_key.verification_key())).await;

        let tx = make_unsigned_tx();
        let signed_tx = Arc::new(tx.sign(&signing_key));

        mempool
            .insert(
                signed_tx.clone(),
                0,
                mock_balances(0, 0),
                mock_tx_cost(0, 0, 0),
            )
            .await
            .unwrap();

        let process_proposal = new_process_proposal_request(&[signed_tx]);
        let txs = process_proposal.txs.clone();
        consensus_service
            .handle_request(ConsensusRequest::ProcessProposal(process_proposal))
            .await
            .unwrap();

        let finalize_block = request::FinalizeBlock {
            hash: Hash::try_from([0u8; 32].to_vec()).unwrap(),
            height: BLOCK_HEIGHT.into(),
            time: Time::now(),
            next_validators_hash: Hash::default(),
            proposer_address: [0u8; 20].to_vec().try_into().unwrap(),
            decided_last_commit: tendermint::abci::types::CommitInfo {
                round: 0u16.into(),
                votes: vec![],
            },
            misbehavior: vec![],
            txs,
        };
        consensus_service
            .handle_request(ConsensusRequest::FinalizeBlock(finalize_block))
            .await
            .unwrap();

        // Mempool should still have a transaction
        assert_eq!(mempool.len().await, 1);
        assert_eq!(mempool.pending_nonce(&address_bytes).await, Some(1));

        let commit = ConsensusRequest::Commit {};
        consensus_service.handle_request(commit).await.unwrap();

        // ensure that txs included in a block are removed from the mempool
        assert_eq!(mempool.len().await, 0);
        assert_eq!(mempool.pending_nonce(&address_bytes).await, None);
    }
}
