use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use anyhow::anyhow;
use futures::{
    Future,
    FutureExt,
};
use penumbra_storage::Storage;
use tendermint::abci::{
    request,
    response,
    ConsensusRequest,
    ConsensusResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

use crate::app::{
    App,
    GenesisState,
};

#[derive(Clone)]
pub struct ConsensusService {
    storage: Storage,
    app: App,
}

impl ConsensusService {
    pub fn new(app: App, storage: Storage) -> Self {
        Self {
            storage,
            app,
        }
    }

    async fn init_chain(
        &mut self,
        init_chain: &request::InitChain,
    ) -> Result<ConsensusResponse, BoxError> {
        // the storage version is set to u64::MAX by default when first created
        if self.storage.latest_version() != u64::MAX {
            return Err(anyhow!("database already initialized").into());
        }

        let genesis_state: GenesisState = serde_json::from_slice(&init_chain.app_state_bytes)
            .expect("can parse app_state in genesis file");

        self.app.init_chain(&genesis_state).await?;

        // TODO: return the genesis app hash
        Ok(ConsensusResponse::InitChain(Default::default()))
    }

    async fn begin_block(
        &mut self,
        begin_block: &request::BeginBlock,
    ) -> Result<ConsensusResponse, BoxError> {
        let events = self.app.begin_block(begin_block).await;
        Ok(ConsensusResponse::BeginBlock(response::BeginBlock {
            events,
        }))
    }

    async fn deliver_tx(&mut self, tx: &[u8]) -> Result<ConsensusResponse, BoxError> {
        self.app.deliver_tx(tx).await?;
        Ok(ConsensusResponse::DeliverTx(Default::default()))
    }

    async fn end_block(
        &mut self,
        end_block: &request::EndBlock,
    ) -> Result<ConsensusResponse, BoxError> {
        let events = self.app.end_block(end_block).await;
        Ok(ConsensusResponse::EndBlock(response::EndBlock {
            events,
            ..Default::default()
        }))
    }

    async fn commit(&mut self) -> Result<ConsensusResponse, BoxError> {
        let app_hash = self.app.commit(self.storage.clone()).await;
        Ok(ConsensusResponse::Commit(response::Commit {
            data: app_hash.0.to_vec().into(),
            ..Default::default()
        }))
    }
}

impl Service<ConsensusRequest> for ConsensusService {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<ConsensusResponse, Self::Error>> + Send>>;
    type Response = ConsensusResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ConsensusRequest) -> Self::Future {
        info!("got consensus request: {:?}", req);
        let mut self2 = self.clone();
        async move {
            match req {
                ConsensusRequest::InitChain(req) => self2.init_chain(&req).await,
                ConsensusRequest::BeginBlock(req) => self2.begin_block(&req).await,
                ConsensusRequest::DeliverTx(req) => self2.deliver_tx(&req.tx).await,
                ConsensusRequest::EndBlock(req) => self2.end_block(&req).await,
                ConsensusRequest::Commit => self2.commit().await,
            }
        }
        .boxed()
    }
}
