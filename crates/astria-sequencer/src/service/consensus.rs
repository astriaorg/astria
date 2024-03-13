use anyhow::{
    bail,
    Context,
};
use astria_core::sequencer::v1alpha1::AbciErrorCode;
use cnidarium::Storage;
use sha2::{
    Digest as _,
    Sha256,
};
use tendermint::v0_37::abci::{
    request,
    response,
    ConsensusRequest,
    ConsensusResponse,
};
use tokio::sync::mpsc;
use tower_abci::BoxError;
use tower_actor::Message;
use tracing::{
    instrument,
    warn,
    Instrument,
};

use crate::{
    app::App,
    genesis::GenesisState,
};

pub(crate) struct Consensus {
    queue: mpsc::Receiver<Message<ConsensusRequest, ConsensusResponse, tower::BoxError>>,
    storage: Storage,
    app: App,
}

impl Consensus {
    pub(crate) fn new(
        storage: Storage,
        app: App,
        queue: mpsc::Receiver<Message<ConsensusRequest, ConsensusResponse, tower::BoxError>>,
    ) -> Self {
        Self {
            queue,
            storage,
            app,
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
            let rsp = self.handle_request(req).instrument(span.clone()).await;
            if let Err(e) = rsp.as_ref() {
                warn!(
                    parent: &span,
                    error = e,
                    "failed processing concensus request; returning error back to sender",
                );
            }
            // `send` returns the sent message if sending fail, so we are dropping it.
            if rsp_sender.send(rsp).is_err() {
                warn!(
                    parent: &span,
                    "failed returning consensus response to request sender; dropping response"
                );
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    async fn handle_request(
        &mut self,
        req: ConsensusRequest,
    ) -> Result<ConsensusResponse, BoxError> {
        Ok(match req {
            ConsensusRequest::InitChain(init_chain) => ConsensusResponse::InitChain(
                self.init_chain(init_chain)
                    .await
                    .context("failed initializing chain")?,
            ),
            ConsensusRequest::PrepareProposal(prepare_proposal) => {
                ConsensusResponse::PrepareProposal(
                    self.handle_prepare_proposal(prepare_proposal)
                        .await
                        .context("failed to prepare proposal")?,
                )
            }
            ConsensusRequest::ProcessProposal(process_proposal) => {
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
                )
            }
            ConsensusRequest::BeginBlock(begin_block) => ConsensusResponse::BeginBlock(
                self.begin_block(begin_block)
                    .await
                    .context("failed to begin block")?,
            ),
            ConsensusRequest::DeliverTx(deliver_tx) => {
                ConsensusResponse::DeliverTx(self.deliver_tx(deliver_tx).await)
            }
            ConsensusRequest::EndBlock(end_block) => ConsensusResponse::EndBlock(
                self.end_block(end_block)
                    .await
                    .context("failed to end block")?,
            ),
            ConsensusRequest::Commit => {
                ConsensusResponse::Commit(self.commit().await.context("failed to commit")?)
            }
        })
    }

    #[instrument(skip_all, fields(
        chain_id = init_chain.chain_id,
        time = %init_chain.time,
        init_height = %init_chain.initial_height
    ))]
    async fn init_chain(
        &mut self,
        init_chain: request::InitChain,
    ) -> anyhow::Result<response::InitChain> {
        // the storage version is set to u64::MAX by default when first created
        if self.storage.latest_version() != u64::MAX {
            bail!("database already initialized");
        }

        let genesis_state: GenesisState = serde_json::from_slice(&init_chain.app_state_bytes)
            .context("failed to parse app_state in genesis file")?;
        self.app
            .init_chain(
                genesis_state,
                init_chain.validators.clone(),
                init_chain.chain_id,
            )
            .await
            .context("failed to call init_chain")?;

        // commit the state and return the app hash
        let app_hash = self.app.commit(self.storage.clone()).await;
        Ok(response::InitChain {
            app_hash: app_hash
                .0
                .to_vec()
                .try_into()
                .context("failed to convert app hash")?,
            consensus_params: Some(init_chain.consensus_params),
            validators: init_chain.validators,
        })
    }

    #[instrument(skip_all, fields(
        height = %prepare_proposal.height,
        tx_count = prepare_proposal.txs.len(),
        time = %prepare_proposal.time
    ))]
    async fn handle_prepare_proposal(
        &mut self,
        prepare_proposal: request::PrepareProposal,
    ) -> anyhow::Result<response::PrepareProposal> {
        self.app
            .prepare_proposal(prepare_proposal, self.storage.clone())
            .await
    }

    #[instrument(skip_all, fields(
        height = %process_proposal.height,
        time = %process_proposal.time,
        tx_count = process_proposal.txs.len(),
        proposer = %process_proposal.proposer_address,
        hash = %telemetry::display::hex(&process_proposal.hash),
        next_validators_hash = %telemetry::display::hex(&process_proposal.next_validators_hash),
    ))]
    async fn handle_process_proposal(
        &mut self,
        process_proposal: request::ProcessProposal,
    ) -> anyhow::Result<()> {
        self.app
            .process_proposal(process_proposal, self.storage.clone())
            .await
    }

    #[instrument(skip_all, fields(
        hash = %begin_block.hash,
        height = %begin_block.header.height,
        time = %begin_block.header.time,
        proposer = %begin_block.header.proposer_address
    ))]
    async fn begin_block(
        &mut self,
        begin_block: request::BeginBlock,
    ) -> anyhow::Result<response::BeginBlock> {
        let events = self
            .app
            .begin_block(&begin_block, self.storage.clone())
            .await
            .context("failed to call App::begin_block")?;
        Ok(response::BeginBlock {
            events,
        })
    }

    #[instrument(skip_all, fields(
        tx_hash = %telemetry::display::hex(&Sha256::digest(&deliver_tx.tx))
    ))]
    async fn deliver_tx(&mut self, deliver_tx: request::DeliverTx) -> response::DeliverTx {
        use crate::transaction::InvalidNonce;

        match self
            .app
            .deliver_tx_after_proposal(deliver_tx)
            .await
            .expect("transactions must be executable or previously executed during proposal phases")
        {
            Ok(events) => response::DeliverTx {
                events,
                ..Default::default()
            },
            Err(e) => {
                let code = if e.downcast_ref::<InvalidNonce>().is_some() {
                    AbciErrorCode::INVALID_NONCE
                } else {
                    AbciErrorCode::INTERNAL_ERROR
                };
                tracing::warn!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed serving deliver tx request"
                );
                response::DeliverTx {
                    code: code.into(),
                    info: code.to_string(),
                    log: format!("{e:?}"),
                    ..Default::default()
                }
            }
        }
    }

    #[instrument(skip_all, fields(height = %end_block.height))]
    async fn end_block(
        &mut self,
        end_block: request::EndBlock,
    ) -> anyhow::Result<response::EndBlock> {
        self.app.end_block(&end_block).await
    }

    #[instrument(skip_all)]
    async fn commit(&mut self) -> anyhow::Result<response::Commit> {
        let app_hash = self.app.commit(self.storage.clone()).await;
        Ok(response::Commit {
            data: app_hash.0.to_vec().into(),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        str::FromStr,
    };

    use astria_core::sequencer::v1alpha1::{
        asset::DEFAULT_NATIVE_ASSET_DENOM,
        transaction::action::SequenceAction,
        Address,
        RollupId,
        UnsignedTransaction,
    };
    use bytes::Bytes;
    use ed25519_consensus::{
        SigningKey,
        VerificationKey,
    };
    use prost::Message as _;
    use rand::rngs::OsRng;
    use tendermint::{
        account::Id,
        Hash,
        Time,
    };

    use super::*;
    use crate::{
        asset::get_native_asset,
        proposal::commitment::generate_rollup_datas_commitment,
    };

    fn make_unsigned_tx() -> UnsignedTransaction {
        UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    rollup_id: RollupId::from_unhashed_bytes(b"testchainid"),
                    data: b"helloworld".to_vec(),
                    fee_asset_id: get_native_asset().id(),
                }
                .into(),
            ],
        }
    }

    fn new_prepare_proposal_request(txs: Vec<Bytes>) -> request::PrepareProposal {
        request::PrepareProposal {
            txs,
            max_tx_bytes: 1024,
            local_last_commit: None,
            misbehavior: vec![],
            height: 1u32.into(),
            time: Time::now(),
            next_validators_hash: Hash::default(),
            proposer_address: Id::from_str("0CDA3F47EF3C4906693B170EF650EB968C5F4B2C").unwrap(),
        }
    }

    fn new_process_proposal_request(txs: Vec<Bytes>) -> request::ProcessProposal {
        request::ProcessProposal {
            txs,
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: Hash::default(),
            height: 1u32.into(),
            next_validators_hash: Hash::default(),
            time: Time::now(),
            proposer_address: Id::from_str("0CDA3F47EF3C4906693B170EF650EB968C5F4B2C").unwrap(),
        }
    }

    #[tokio::test]
    async fn prepare_and_process_proposal() {
        let signing_key = SigningKey::new(OsRng);
        let mut consensus_service =
            new_consensus_service(Some(signing_key.verification_key())).await;
        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.clone().into_raw().encode_to_vec();
        let txs = vec![tx_bytes.into()];

        let res = generate_rollup_datas_commitment(&vec![signed_tx], HashMap::new());

        let prepare_proposal = new_prepare_proposal_request(txs.clone());
        let prepare_proposal_response = consensus_service
            .handle_prepare_proposal(prepare_proposal)
            .await
            .unwrap();
        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: res.into_transactions(txs)
            }
        );

        let mut consensus_service =
            new_consensus_service(Some(signing_key.verification_key())).await;
        let process_proposal = new_process_proposal_request(prepare_proposal_response.txs);
        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn process_proposal_ok() {
        let signing_key = SigningKey::new(OsRng);
        let mut consensus_service =
            new_consensus_service(Some(signing_key.verification_key())).await;
        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.clone().into_raw().encode_to_vec();
        let txs = vec![tx_bytes.into()];
        let res = generate_rollup_datas_commitment(&vec![signed_tx], HashMap::new());
        let process_proposal = new_process_proposal_request(res.into_transactions(txs));
        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn process_proposal_fail_missing_action_commitment() {
        let mut consensus_service = new_consensus_service(None).await;
        let process_proposal = new_process_proposal_request(vec![]);
        assert!(
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("no transaction commitment in proposal")
        );
    }

    #[tokio::test]
    async fn process_proposal_fail_wrong_commitment_length() {
        let mut consensus_service = new_consensus_service(None).await;
        let process_proposal = new_process_proposal_request(vec![[0u8; 16].to_vec().into()]);
        assert!(
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("transaction commitment must be 32 bytes")
        );
    }

    #[tokio::test]
    async fn process_proposal_fail_wrong_commitment_value() {
        let mut consensus_service = new_consensus_service(None).await;
        let process_proposal = new_process_proposal_request(vec![
            [99u8; 32].to_vec().into(),
            [99u8; 32].to_vec().into(),
        ]);
        assert!(
            consensus_service
                .handle_process_proposal(process_proposal)
                .await
                .err()
                .unwrap()
                .to_string()
                .contains("transaction commitment does not match expected")
        );
    }

    #[tokio::test]
    async fn prepare_proposal_empty_block() {
        let mut consensus_service = new_consensus_service(None).await;
        let txs = vec![];
        let res = generate_rollup_datas_commitment(&txs.clone(), HashMap::new());
        let prepare_proposal = new_prepare_proposal_request(vec![]);

        let prepare_proposal_response = consensus_service
            .handle_prepare_proposal(prepare_proposal)
            .await
            .unwrap();
        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: res.into_transactions(vec![]),
            }
        );
    }

    #[tokio::test]
    async fn process_proposal_ok_empty_block() {
        let mut consensus_service = new_consensus_service(None).await;
        let txs = vec![];
        let res = generate_rollup_datas_commitment(&txs, HashMap::new());
        let process_proposal = new_process_proposal_request(res.into_transactions(vec![]));
        consensus_service
            .handle_process_proposal(process_proposal)
            .await
            .unwrap();
    }

    /// Returns a default tendermint block header for test purposes.
    fn default_header() -> tendermint::block::Header {
        use tendermint::{
            account,
            block::{
                header::Version,
                Height,
            },
            chain,
            hash::AppHash,
        };

        tendermint::block::Header {
            version: Version {
                block: 0,
                app: 0,
            },
            chain_id: chain::Id::try_from("test").unwrap(),
            height: Height::from(1u32),
            time: Time::now(),
            last_block_id: None,
            last_commit_hash: None,
            data_hash: None,
            validators_hash: Hash::Sha256([0; 32]),
            next_validators_hash: Hash::Sha256([0; 32]),
            consensus_hash: Hash::Sha256([0; 32]),
            app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
            last_results_hash: None,
            evidence_hash: None,
            proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
        }
    }

    impl Default for GenesisState {
        fn default() -> Self {
            Self {
                accounts: vec![],
                authority_sudo_address: Address::from([0; 20]),
                ibc_sudo_address: Address::from([0; 20]),
                ibc_relayer_addresses: vec![],
                native_asset_base_denomination: DEFAULT_NATIVE_ASSET_DENOM.to_string(),
                ibc_params: penumbra_ibc::params::IBCParameters::default(),
                allowed_fee_assets: vec![DEFAULT_NATIVE_ASSET_DENOM.to_owned().into()],
            }
        }
    }

    async fn new_consensus_service(funded_key: Option<VerificationKey>) -> Consensus {
        let accounts = if funded_key.is_some() {
            vec![crate::genesis::Account {
                address: Address::from_verification_key(funded_key.unwrap()),
                balance: 10u128.pow(19),
            }]
        } else {
            vec![]
        };
        let genesis_state = GenesisState {
            accounts,
            ..Default::default()
        };

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut app = App::new(snapshot);
        app.init_chain(genesis_state, vec![], "test".to_string())
            .await
            .unwrap();
        app.commit(storage.clone()).await;

        let (_tx, rx) = mpsc::channel(1);
        Consensus::new(storage.clone(), app, rx)
    }

    #[tokio::test]
    async fn block_lifecycle() {
        let signing_key = SigningKey::new(OsRng);
        let mut consensus_service =
            new_consensus_service(Some(signing_key.verification_key())).await;

        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.clone().into_raw().encode_to_vec();
        let txs = vec![tx_bytes.clone().into()];
        let res = generate_rollup_datas_commitment(&vec![signed_tx], HashMap::new());

        let block_data = res.into_transactions(txs.clone());
        let data_hash = merkle::Tree::from_leaves(block_data.iter().map(Sha256::digest)).root();
        let mut header = default_header();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let process_proposal = new_process_proposal_request(block_data.clone());
        consensus_service
            .handle_request(ConsensusRequest::ProcessProposal(process_proposal))
            .await
            .unwrap();

        let begin_block = request::BeginBlock {
            hash: Hash::default(),
            header,
            last_commit_info: tendermint::abci::types::CommitInfo {
                round: 0u16.into(),
                votes: vec![],
            },
            byzantine_validators: vec![],
        };
        consensus_service
            .handle_request(ConsensusRequest::BeginBlock(begin_block))
            .await
            .unwrap();

        for tx in block_data {
            let deliver_tx = request::DeliverTx {
                tx,
            };
            consensus_service
                .handle_request(ConsensusRequest::DeliverTx(deliver_tx))
                .await
                .unwrap();
        }

        let end_block = request::EndBlock {
            height: 1u32.into(),
        };
        consensus_service
            .handle_request(ConsensusRequest::EndBlock(end_block))
            .await
            .unwrap();
        consensus_service
            .handle_request(ConsensusRequest::Commit)
            .await
            .unwrap();
    }
}
