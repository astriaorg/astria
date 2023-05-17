use std::{
    pin::Pin,
    sync::Arc,
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
    app: Arc<App>,
}

impl ConsensusService {
    pub fn new(app: App, storage: Storage) -> Self {
        Self {
            storage,
            app: Arc::new(app),
        }
    }

    async fn init_chain(
        storage: Storage,
        mut app: Arc<App>,
        init_chain: request::InitChain,
    ) -> Result<ConsensusResponse, BoxError> {
        // the storage version is set to u64::MAX by default when first created
        if storage.latest_version() != u64::MAX {
            return Err(anyhow!("database already initialized").into());
        }

        let genesis_state: GenesisState = serde_json::from_slice(&init_chain.app_state_bytes)
            .expect("can parse app_state in genesis file");

        let app = Arc::get_mut(&mut app).expect("no other references to App");
        app.init_chain(&genesis_state).await?;

        // TODO: return the genesis app hash
        Ok(ConsensusResponse::InitChain(Default::default()))
    }

    async fn begin_block(
        mut app: Arc<App>,
        begin_block: request::BeginBlock,
    ) -> Result<ConsensusResponse, BoxError> {
        println!(
            "ConsensusService::begin_block {}",
            Arc::<App>::strong_count(&app)
        );

        let app = Arc::get_mut(&mut app).expect("no other references to App");
        let events = app.begin_block(&begin_block).await;
        Ok(ConsensusResponse::BeginBlock(response::BeginBlock {
            events,
        }))
    }

    async fn deliver_tx(
        mut app: Arc<App>,
        tx: bytes::Bytes,
    ) -> Result<ConsensusResponse, BoxError> {
        let app = Arc::get_mut(&mut app).expect("no other references to App");
        app.deliver_tx(&tx).await?;
        Ok(ConsensusResponse::DeliverTx(Default::default()))
    }

    async fn end_block(
        mut app: Arc<App>,
        end_block: request::EndBlock,
    ) -> Result<ConsensusResponse, BoxError> {
        let app = Arc::get_mut(&mut app).expect("no other references to App");
        let events = app.end_block(&end_block).await;
        Ok(ConsensusResponse::EndBlock(response::EndBlock {
            events,
            ..Default::default()
        }))
    }

    async fn commit(storage: Storage, mut app: Arc<App>) -> Result<ConsensusResponse, BoxError> {
        let app = Arc::get_mut(&mut app).expect("no other references to App");
        let app_hash = app.commit(storage.clone()).await;
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
        println!(
            "ConsensusService::call 0 {}",
            Arc::<App>::strong_count(&self.app)
        );

        let storage = self.storage.clone();
        let app = self.app.clone();

        println!(
            "ConsensusService::call 1 {}",
            Arc::<App>::strong_count(&self.app)
        );

        match req {
            ConsensusRequest::InitChain(req) => {
                ConsensusService::init_chain(storage, app, req).boxed()
            }
            ConsensusRequest::BeginBlock(req) => ConsensusService::begin_block(app, req).boxed(),
            ConsensusRequest::DeliverTx(req) => ConsensusService::deliver_tx(app, req.tx).boxed(),
            ConsensusRequest::EndBlock(req) => {
                ConsensusService::end_block(app, req.clone()).boxed()
            }
            ConsensusRequest::Commit => ConsensusService::commit(storage, app).boxed(),
        }
    }
}
