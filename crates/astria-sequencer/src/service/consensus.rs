use anyhow::{
    anyhow,
    bail,
    Context,
};
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
                warn!(parent: &span, "failed returning consensus response to request sender; dropping response");
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
                ConsensusResponse::PrepareProposal(response::PrepareProposal {
                    txs: prepare_proposal.txs,
                })
            }
            ConsensusRequest::ProcessProposal(_process_proposal) => {
                // TODO: handle this
                ConsensusResponse::ProcessProposal(response::ProcessProposal::Accept)
            }
            ConsensusRequest::BeginBlock(begin_block) => {
                ConsensusResponse::BeginBlock(
                    self.begin_block(begin_block)
                        .await
                        // .context() cannot be used here because Box<dyn Error> does not implement
                        // Error: https://github.com/dtolnay/anyhow/issues/66
                        .map_err(|e| anyhow!(e))
                        .context("failed to begin block")?,
                )
            }
            ConsensusRequest::DeliverTx(deliver_tx) => ConsensusResponse::DeliverTx(
                self.deliver_tx(deliver_tx)
                    .await
                    // .context() cannot be used here because Box<dyn Error> does not implement
                    // Error: https://github.com/dtolnay/anyhow/issues/66
                    .map_err(|e| anyhow!(e))
                    .context("failed to deliver transaction")?,
            ),
            ConsensusRequest::EndBlock(end_block) => ConsensusResponse::EndBlock(
                self.end_block(end_block)
                    .await
                    .map_err(|e| anyhow!(e))
                    .context("failed to end block")?,
            ),
            ConsensusRequest::Commit => ConsensusResponse::Commit(
                self.commit()
                    .await
                    .map_err(|e| anyhow!(e))
                    .context("failed to commit")?,
            ),
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
    ) -> Result<response::BeginBlock, BoxError> {
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
    ) -> Result<response::DeliverTx, BoxError> {
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
    ) -> Result<response::EndBlock, BoxError> {
        let events = self.app.end_block(&end_block).await;
        Ok(response::EndBlock {
            events,
            ..Default::default()
        })
    }

    #[instrument(skip(self))]
    async fn commit(&mut self) -> Result<response::Commit, BoxError> {
        let app_hash = self.app.commit(self.storage.clone()).await;
        Ok(response::Commit {
            data: app_hash.0.to_vec().into(),
            ..Default::default()
        })
    }
}
