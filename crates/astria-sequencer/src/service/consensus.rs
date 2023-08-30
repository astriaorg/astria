use std::collections::VecDeque;

use anyhow::{
    anyhow,
    bail,
    ensure,
    Context,
};
use bytes::Bytes;
use penumbra_storage::Storage;
use tendermint::abci::{
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
    proposal::commitment::generate_sequence_actions_commitment,
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
                warn!(parent: &span, error = ?e, "failed processing concensus request; returning error back to sender");
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

    #[instrument(skip(self))]
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
                ConsensusResponse::PrepareProposal(handle_prepare_proposal(prepare_proposal))
            }
            ConsensusRequest::ProcessProposal(process_proposal) => {
                ConsensusResponse::ProcessProposal(
                    match handle_process_proposal(process_proposal) {
                        Ok(()) => response::ProcessProposal::Accept,
                        Err(e) => {
                            warn!(error = ?e, "rejecting proposal");
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
            ConsensusRequest::DeliverTx(deliver_tx) => ConsensusResponse::DeliverTx(
                self.deliver_tx(deliver_tx)
                    .await
                    .context("failed to deliver transaction")?,
            ),
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

    #[instrument(skip(self))]
    async fn init_chain(
        &mut self,
        init_chain: request::InitChain,
    ) -> anyhow::Result<response::InitChain> {
        // the storage version is set to u64::MAX by default when first created
        if self.storage.latest_version() != u64::MAX {
            bail!("database already initialized");
        }

        let genesis_state: GenesisState = serde_json::from_slice(&init_chain.app_state_bytes)
            .expect("can parse app_state in genesis file");

        self.app.init_chain(genesis_state).await?;

        // TODO: return the genesis app hash
        Ok(response::InitChain::default())
    }

    #[instrument(skip(self))]
    async fn begin_block(
        &mut self,
        begin_block: request::BeginBlock,
    ) -> anyhow::Result<response::BeginBlock> {
        if self.storage.latest_version() == u64::MAX {
            // TODO: why isn't tendermint calling init_chain before the first block?
            self.app
                .init_chain(GenesisState::default())
                .await
                .expect("init_chain must succeed");
        }

        let events = self.app.begin_block(&begin_block).await;
        Ok(response::BeginBlock {
            events,
        })
    }

    #[instrument(skip(self))]
    async fn deliver_tx(
        &mut self,
        deliver_tx: request::DeliverTx,
    ) -> anyhow::Result<response::DeliverTx> {
        self.app
            .deliver_tx(&deliver_tx.tx)
            .await
            .unwrap_or_else(|e| {
                // we don't want to panic on failing to deliver_tx as that would crash the entire
                // node
                tracing::error!(error = ?e, "deliver_tx failed");
                vec![]
            });
        Ok(response::DeliverTx::default())
    }

    #[instrument(skip(self))]
    async fn end_block(
        &mut self,
        end_block: request::EndBlock,
    ) -> anyhow::Result<response::EndBlock> {
        let events = self.app.end_block(&end_block).await;
        Ok(response::EndBlock {
            events,
            ..Default::default()
        })
    }

    #[instrument(skip(self))]
    async fn commit(&mut self) -> anyhow::Result<response::Commit> {
        let app_hash = self.app.commit(self.storage.clone()).await;
        Ok(response::Commit {
            data: app_hash.0.to_vec().into(),
            ..Default::default()
        })
    }
}

/// Generates a commitment to the `sequence::Actions` in the block's transactions.
/// This is required so that a rollup can easily verify that the transactions it
/// receives are correct (ie. we actually included in a sequencer block, and none
/// are missing)
/// It puts this special "commitment" as the first transaction in a block.
/// When other validators receive the block, they know the first transaction is
/// supposed to be the commitment, and verifies that is it correct.
#[instrument]
fn handle_prepare_proposal(
    prepare_proposal: request::PrepareProposal,
) -> response::PrepareProposal {
    let (action_commitment, mut txs_to_be_included) =
        generate_sequence_actions_commitment(prepare_proposal.txs);
    let mut txs: Vec<Bytes> = vec![action_commitment.to_vec().into()];
    txs.append(&mut txs_to_be_included);
    response::PrepareProposal {
        txs,
    }
}

/// Generates a commitment to the `sequence::Actions` in the block's transactions
/// and ensures it matches the commitment created by the proposer, which
/// should be the first transaction in the block.
#[instrument]
fn handle_process_proposal(process_proposal: request::ProcessProposal) -> anyhow::Result<()> {
    let mut txs = VecDeque::from(process_proposal.txs);
    let received_action_commitment: [u8; 32] = txs
        .pop_front()
        .context("no transaction commitment in proposal")?
        .to_vec()
        .try_into()
        .map_err(|_| anyhow!("transaction commitment must be 32 bytes"))?;
    let expected_txs_len = txs.len();

    let (expected_action_commitment, txs_to_be_included) =
        generate_sequence_actions_commitment(txs.into());
    ensure!(
        received_action_commitment == expected_action_commitment,
        "transaction commitment does not match expected",
    );

    // all txs in the proposal should be deserializable
    ensure!(
        txs_to_be_included.len() == expected_txs_len,
        "transactions to be included do not match expected",
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use ed25519_consensus::SigningKey;
    use proto::{
        native::sequencer::v1alpha1::{
            SequenceAction,
            UnsignedTransaction,
        },
        Message as _,
    };
    use rand::rngs::OsRng;
    use tendermint::{
        account::Id,
        Hash,
        Time,
    };

    use super::*;

    fn make_unsigned_tx() -> UnsignedTransaction {
        UnsignedTransaction {
            nonce: 0,
            actions: vec![
                SequenceAction {
                    chain_id: b"testchainid".to_vec(),
                    data: b"helloworld".to_vec(),
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

    #[test]
    fn prepare_and_process_proposal() {
        let signing_key = SigningKey::new(OsRng);
        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();

        let txs = vec![tx_bytes.clone().into()];
        let (action_commitment, txs_included) = generate_sequence_actions_commitment(txs.clone());
        assert_eq!(txs, txs_included);

        let prepare_proposal = new_prepare_proposal_request(txs_included);
        let prepare_proposal_response = handle_prepare_proposal(prepare_proposal);
        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: vec![action_commitment.to_vec().into(), tx_bytes.into()],
            }
        );

        let process_proposal = new_process_proposal_request(prepare_proposal_response.txs);
        handle_process_proposal(process_proposal).unwrap();
    }

    #[test]
    fn process_proposal_ok() {
        let signing_key = SigningKey::new(OsRng);
        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();
        let txs = vec![tx_bytes.clone().into()];
        let (action_commitment, txs_included) = generate_sequence_actions_commitment(txs.clone());
        assert_eq!(txs, txs_included);

        let process_proposal =
            new_process_proposal_request(vec![action_commitment.to_vec().into(), tx_bytes.into()]);
        handle_process_proposal(process_proposal).unwrap();
    }

    #[test]
    fn process_proposal_fail_missing_action_commitment() {
        let process_proposal = new_process_proposal_request(vec![]);
        assert!(
            handle_process_proposal(process_proposal)
                .err()
                .unwrap()
                .to_string()
                .contains("no transaction commitment in proposal")
        );
    }

    #[test]
    fn process_proposal_fail_wrong_commitment_length() {
        let process_proposal = new_process_proposal_request(vec![[0u8; 16].to_vec().into()]);
        assert!(
            handle_process_proposal(process_proposal)
                .err()
                .unwrap()
                .to_string()
                .contains("transaction commitment must be 32 bytes")
        );
    }

    #[test]
    fn process_proposal_fail_wrong_commitment_value() {
        let process_proposal = new_process_proposal_request(vec![[99u8; 32].to_vec().into()]);
        assert!(
            handle_process_proposal(process_proposal)
                .err()
                .unwrap()
                .to_string()
                .contains("transaction commitment does not match expected")
        );
    }

    #[test]
    fn prepare_proposal_empty_block() {
        let txs = vec![];
        let (action_commitment, txs_included) = generate_sequence_actions_commitment(txs.clone());
        assert_eq!(txs, txs_included);
        let prepare_proposal = new_prepare_proposal_request(txs_included);

        let prepare_proposal_response = handle_prepare_proposal(prepare_proposal);
        assert_eq!(
            prepare_proposal_response,
            response::PrepareProposal {
                txs: vec![action_commitment.to_vec().into()],
            }
        );
    }

    #[test]
    fn process_proposal_ok_empty_block() {
        let txs = vec![];
        let (action_commitment, _) = generate_sequence_actions_commitment(txs);
        let process_proposal =
            new_process_proposal_request(vec![action_commitment.to_vec().into()]);
        handle_process_proposal(process_proposal).unwrap();
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

    async fn new_consensus_service() -> Consensus {
        let storage = penumbra_storage::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let app = App::new(snapshot);

        let (_tx, rx) = mpsc::channel(1);
        Consensus::new(storage.clone(), app, rx)
    }

    #[tokio::test]
    async fn block_lifecycle() {
        let mut consensus_service = new_consensus_service().await;

        let signing_key = SigningKey::new(OsRng);
        let tx = make_unsigned_tx();
        let signed_tx = tx.into_signed(&signing_key);
        let tx_bytes = signed_tx.into_raw().encode_to_vec();
        let txs = vec![tx_bytes.clone().into()];
        let (action_commitment, txs_included) = generate_sequence_actions_commitment(txs.clone());
        assert_eq!(txs, txs_included);

        let txs = vec![action_commitment.to_vec().into(), tx_bytes.into()];
        let process_proposal = new_process_proposal_request(txs.clone());
        consensus_service
            .handle_request(ConsensusRequest::ProcessProposal(process_proposal))
            .await
            .unwrap();

        let begin_block = request::BeginBlock {
            hash: Hash::default(),
            header: default_header(),
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

        for tx in txs {
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
