//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use std::fmt;

use astria_sequencer_types::SequencerBlockData;
use color_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};
// use futures::{
//     io::empty,
//     StreamExt,
// };
use sequencer_client::{
    tendermint,
    NewBlockStreamError,
    WebSocketClient,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            UnboundedReceiver,
            UnboundedSender,
        },
        Mutex,
    },
    task::JoinHandle,
};
use tracing::{
    info,
    instrument,
    span,
    warn,
    Instrument,
    Level,
};

use crate::{
    block_verifier::BlockVerifier,
    config::{
        CommitLevel,
        Config,
    },
    executor,
    executor::ExecutorCommand,
    reader::{
        self,
        ReaderCommand,
    },
};

/// The channel through which the user can send commands to the driver.
pub(crate) type Sender = UnboundedSender<DriverCommand>;
/// The channel on which the driver listens for commands from the user.
pub(crate) type Receiver = UnboundedReceiver<DriverCommand>;

/// The type of commands that the driver can receive.
#[derive(Debug)]
pub(crate) enum DriverCommand {
    /// Get new blocks
    GetNewBlocks,
    /// Gracefully shuts down the driver and its components.
    Shutdown,
}

#[derive(Debug)]
pub(crate) struct Driver {
    pub(crate) cmd_tx: Sender,

    /// The channel on which other components in the driver sends the driver messages.
    cmd_rx: Receiver,

    /// The channel used to send messages to the reader task.
    reader_tx: Option<reader::Sender>,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// A client that subscribes to new sequencer blocks from cometbft.
    // TODO: update to option
    // sequencer_client: SequencerClient,
    sequencer_client: Option<SequencerClient>,

    is_shutdown: Mutex<bool>,
}

struct SequencerClient {
    client: WebSocketClient,
    _driver: JoinHandle<Result<(), tendermint::Error>>,
}

impl SequencerClient {
    async fn new(url: &str) -> Result<Self, tendermint::Error> {
        let (client, driver) = WebSocketClient::new(url).await?;
        Ok(Self {
            client,
            _driver: tokio::spawn(async move { driver.run().await }),
        })
    }
}

impl fmt::Debug for SequencerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SequencerClient")
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl Driver {
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn new(
        conf: Config,
    ) -> Result<(Self, executor::JoinHandle, Option<reader::JoinHandle>)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let executor_span = span!(Level::ERROR, "executor::spawn");
        let (executor_join_handle, executor_tx) = executor::spawn(&conf)
            .instrument(executor_span)
            .await
            .wrap_err("failed to construct Executor")?;

        let block_verifier = BlockVerifier::new(&conf.tendermint_url)
            .wrap_err("failed to construct block verifier")?;

        let (reader_join_handle, reader_tx) = match conf.execution_commit_level {
            CommitLevel::SoftOnly => (None, None),
            CommitLevel::FirmOnly | CommitLevel::SoftAndFirm => {
                let reader_span = span!(Level::ERROR, "reader::spawn");
                let (reader_join_handle, reader_tx) =
                    reader::spawn(&conf, executor_tx.clone(), block_verifier)
                        .instrument(reader_span)
                        .await
                        .wrap_err("failed to construct data availability Reader")?;
                (Some(reader_join_handle), Some(reader_tx))
            }
        };

        let sequencer_client = match conf.execution_commit_level {
            CommitLevel::SoftOnly | CommitLevel::SoftAndFirm => {
                let sequencer_client = SequencerClient::new(&conf.sequencer_url).await.wrap_err(
                    "failed constructing a cometbft websocket client to read off sequencer",
                )?;
                Some(sequencer_client)
            }
            CommitLevel::FirmOnly => None,
        };

        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                reader_tx,
                executor_tx,
                sequencer_client,
                is_shutdown: Mutex::new(false),
            },
            executor_join_handle,
            reader_join_handle,
        ))
    }

    /// Runs the Driver event loop.
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn run(mut self) -> Result<()> {
        use futures::StreamExt as _;
        use sequencer_client::{
            extension_trait::NewBlocksStream,
            SequencerSubscriptionClientExt as _,
        };

        info!("Starting driver event loop.");
        let mut new_blocks = if self.sequencer_client.is_some() {
            let seq_client = self.sequencer_client.take().unwrap();
            seq_client
                .client
                .subscribe_new_block_data()
                .await
                .wrap_err("failed subscribing to sequencer to receive new blocks")?
        } else {
            NewBlocksStream::empty()
        };
        // FIXME(https://github.com/astriaorg/astria/issues/381): the event handlers
        // here block the select loop because they `await` their return.
        loop {
            select! {
                new_block = new_blocks.next() => {
                    if let Some(block) = new_block {
                        self.handle_new_block(block).await
                    } else {
                        warn!("sequencer new-block subscription closed unexpectedly; shutting down driver");
                        break;
                    }
                }
                cmd = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        self.handle_driver_command(cmd).await.wrap_err("failed to handle driver command")?;
                    } else {
                        info!("Driver command channel closed.");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_new_block(&self, block: Result<SequencerBlockData, NewBlockStreamError>) {
        let block = match block {
            Err(err) => {
                warn!(err.msg = %err, err.cause = ?err, "encountered an error while receiving a new block from sequencer");
                return;
            }
            Ok(new_block) => new_block,
        };

        if let Err(err) = self
            .executor_tx
            .send(ExecutorCommand::BlockReceivedFromSequencer {
                block: Box::new(block),
            })
        {
            warn!(err.msg = %err, err.cause = ?err, "failed sending new block received from sequencer to executor");
        }
    }

    async fn handle_driver_command(&mut self, cmd: DriverCommand) -> Result<()> {
        match cmd {
            DriverCommand::Shutdown => {
                self.shutdown().await?;
            }

            DriverCommand::GetNewBlocks => {
                let Some(reader_tx) = &self.reader_tx else {
                    return Ok(());
                };

                reader_tx
                    .send(ReaderCommand::GetNewBlocks)
                    .map_err(|e| eyre!("reader rx channel closed: {}", e))?;
            }
        }

        Ok(())
    }

    /// Sends shutdown commands to the other actors.
    async fn shutdown(&mut self) -> Result<()> {
        let mut is_shutdown = self.is_shutdown.lock().await;
        if *is_shutdown {
            return Ok(());
        }
        *is_shutdown = true;

        info!("Shutting down driver.");
        self.executor_tx.send(ExecutorCommand::Shutdown)?;

        let Some(reader_tx) = &self.reader_tx else {
            return Ok(());
        };
        reader_tx.send(ReaderCommand::Shutdown)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {

    use test_utils::{
        get_test_config,
        MockCelestia,
        MockExecution,
        MockSequencer,
    };

    use super::*;
    use crate::config::CommitLevel;

    #[tokio::test]
    async fn new_driver_execution_commit_level_set_to_soft_only() {
        let mut config = get_test_config();
        config.execution_commit_level = CommitLevel::SoftOnly;

        let execution_server = MockExecution::spawn().await;
        config.execution_rpc_url = execution_server.local_addr();

        let sequencer = MockSequencer::spawn().await;
        config.sequencer_url = sequencer.local_addr();

        let (driver, ..) = Driver::new(config).await.unwrap();

        assert!(driver.reader_tx.is_none());
        assert!(driver.sequencer_client.is_some());
    }

    #[tokio::test]
    async fn new_driver_execution_commit_level_set_to_firm_only() {
        let mut config = get_test_config();
        config.execution_commit_level = CommitLevel::FirmOnly;

        let execution_server = MockExecution::spawn().await;
        config.execution_rpc_url = execution_server.local_addr();

        let mut celestia = MockCelestia::spawn().await;
        config.celestia_node_url = celestia.local_addr();

        let (driver, ..) = Driver::new(config).await.unwrap();

        assert!(driver.reader_tx.is_some());
        assert!(driver.sequencer_client.is_none());
    }

    #[tokio::test]
    async fn new_driver_execution_commit_level_set_to_soft_and_firm() {
        let mut config = get_test_config();
        config.execution_commit_level = CommitLevel::SoftAndFirm;

        let execution_server = MockExecution::spawn().await;
        config.execution_rpc_url = execution_server.local_addr();

        let sequencer = MockSequencer::spawn().await;
        config.sequencer_url = sequencer.local_addr();

        let mut celestia = MockCelestia::spawn().await;
        config.celestia_node_url = celestia.local_addr();

        let (driver, ..) = Driver::new(config).await.unwrap();

        assert!(driver.reader_tx.is_some());
        assert!(driver.sequencer_client.is_some());
    }
}

#[cfg(test)]
pub(crate) mod test_utils {

    use std::{
        net::SocketAddr,
        time::Duration,
    };

    use astria_celestia_jsonrpc_client::rpc_impl::{
        blob::Blob,
        header::HeaderServer,
        state::{
            Fee,
            StateServer,
        },
    };
    use astria_proto::generated::execution::v1alpha1::{
        execution_service_server::{
            ExecutionService,
            ExecutionServiceServer,
        },
        DoBlockRequest,
        DoBlockResponse,
        FinalizeBlockRequest,
        FinalizeBlockResponse,
        InitStateRequest,
        InitStateResponse,
    };
    use ethers::types::Transaction;
    use jsonrpsee::{
        core::{
            async_trait,
            SubscriptionResult,
        },
        proc_macros::rpc,
        server::{
            IdProvider,
            ServerHandle,
        },
        types::{
            ErrorObjectOwned,
            SubscriptionId,
        },
        PendingSubscriptionSink,
    };
    use prost_types::Timestamp;
    use serde::Deserialize;
    use sha2::Digest as _;
    use tokio::sync::{
        broadcast::{
            channel,
            Sender,
        },
        oneshot,
    };
    use tonic::transport::Server;

    use super::*;
    use crate::{
        config,
        execution_client::ExecutionClient,
    };

    // generate a test config
    pub(crate) fn get_test_config() -> Config {
        Config {
            chain_id: "ethereum".to_string(),
            execution_rpc_url: "http://127.0.0.1:50051".to_string(),
            log: "info".to_string(),
            disable_empty_block_execution: false,
            celestia_node_url: "http://127.0.0.1:26659".to_string(),
            celestia_bearer_token: "test".to_string(),
            tendermint_url: "http://127.0.0.1:26657".to_string(),
            sequencer_url: "ws://127.0.0.1:26657".to_string(),
            execution_commit_level: config::CommitLevel::SoftAndFirm,
        }
    }

    // the mock execution server
    #[derive(Debug, Default)]
    struct MockExecutionServer {}

    #[async_trait::async_trait]
    impl ExecutionClient for MockExecutionServer {
        async fn call_init_state(&mut self) -> Result<InitStateResponse> {
            unimplemented!("call_init_state")
        }

        async fn call_do_block(
            &mut self,
            _prev_block_hash: Vec<u8>,
            _transactions: Vec<Vec<u8>>,
            _timestamp: Option<Timestamp>,
        ) -> Result<DoBlockResponse> {
            unimplemented!("call_do_block")
        }

        async fn call_finalize_block(&mut self, _block_hash: Vec<u8>) -> Result<()> {
            unimplemented!("call_finalize_block")
        }
    }

    #[tonic::async_trait]
    impl ExecutionService for MockExecutionServer {
        async fn init_state(
            &self,
            _request: tonic::Request<InitStateRequest>,
        ) -> std::result::Result<tonic::Response<InitStateResponse>, tonic::Status> {
            let hasher = sha2::Sha256::new();
            Ok(tonic::Response::new(InitStateResponse {
                block_hash: hasher.finalize().to_vec(),
            }))
        }

        async fn do_block(
            &self,
            _request: tonic::Request<DoBlockRequest>,
        ) -> std::result::Result<tonic::Response<DoBlockResponse>, tonic::Status> {
            unimplemented!("do_block")
        }

        async fn finalize_block(
            &self,
            _request: tonic::Request<FinalizeBlockRequest>,
        ) -> std::result::Result<tonic::Response<FinalizeBlockResponse>, tonic::Status> {
            unimplemented!("finalize_block")
        }
    }

    pub(crate) struct MockExecution {
        pub(crate) _server_handle: JoinHandle<()>,
        pub(crate) addr: SocketAddr,
    }

    impl MockExecution {
        pub(crate) async fn spawn() -> Self {
            use tokio::net::TcpListener;
            // randomly generating a local address
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            drop(listener);

            let server_handle = tokio::spawn(async move {
                let _ = Server::builder()
                    .add_service(ExecutionServiceServer::new(MockExecutionServer::default()))
                    .serve(addr)
                    .await;
            });
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            Self {
                _server_handle: server_handle,
                addr,
            }
        }

        pub(crate) fn local_addr(&self) -> String {
            format!("http://{}", self.addr)
        }
    }

    // the mock celestia server
    #[allow(dead_code)]
    pub(crate) enum CelestiaMode {
        Immediate,
        Delayed(u64),
    }

    pub(crate) struct MockCelestia {
        pub(crate) addr_rx: oneshot::Receiver<SocketAddr>,
        pub(crate) _state_rpc_confirmed_rx: mpsc::UnboundedReceiver<Vec<Blob>>,
        pub(crate) _server_handle: ServerHandle,
    }

    impl MockCelestia {
        pub(crate) async fn spawn() -> Self {
            let block_time = 1000;
            let mode = CelestiaMode::Immediate;
            Self::start(block_time, mode).await
        }

        async fn start(sequencer_block_time_ms: u64, mode: CelestiaMode) -> Self {
            use jsonrpsee::server::ServerBuilder;
            let (addr_tx, addr_rx) = oneshot::channel();
            let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
            let addr = server.local_addr().unwrap();
            addr_tx.send(addr).unwrap();
            let (state_rpc_confirmed_tx, state_rpc_confirmed_rx) = mpsc::unbounded_channel();
            let state_celestia = StateCelestiaImpl {
                sequencer_block_time_ms,
                mode,
                rpc_confirmed_tx: state_rpc_confirmed_tx,
            };
            let header_celestia = HeaderCelestiaImpl {};
            let mut merged_celestia = state_celestia.into_rpc();
            merged_celestia.merge(header_celestia.into_rpc()).unwrap();
            let _server_handle = server.start(merged_celestia);
            Self {
                addr_rx,
                _state_rpc_confirmed_rx: state_rpc_confirmed_rx,
                _server_handle,
            }
        }

        pub(crate) fn local_addr(&mut self) -> String {
            format!("http://{}", self.addr_rx.try_recv().unwrap())
        }
    }

    struct HeaderCelestiaImpl;

    #[async_trait]
    impl HeaderServer for HeaderCelestiaImpl {
        async fn network_head(&self) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned> {
            use astria_celestia_jsonrpc_client::header::{
                Commit,
                NetworkHeaderResponse,
            };
            use serde_json::{
                to_string,
                value::RawValue,
                Value,
            };
            let rsp = RawValue::from_string(
                to_string(&NetworkHeaderResponse {
                    commit: Commit {
                        height: 42,
                        rest: Value::default(),
                    },
                    inner: Value::default(),
                })
                .unwrap(),
            )
            .unwrap();
            Ok(rsp)
        }
    }

    struct StateCelestiaImpl {
        sequencer_block_time_ms: u64,
        mode: CelestiaMode,
        rpc_confirmed_tx: mpsc::UnboundedSender<Vec<Blob>>,
    }

    #[async_trait]
    impl StateServer for StateCelestiaImpl {
        async fn submit_pay_for_blob(
            &self,
            _fee: Fee,
            _gas_limit: u64,
            blobs: Vec<Blob>,
        ) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned> {
            use astria_celestia_jsonrpc_client::state::SubmitPayForBlobResponse;
            use serde_json::{
                to_string,
                value::RawValue,
                Value,
            };

            self.rpc_confirmed_tx.send(blobs).unwrap();

            let rsp = RawValue::from_string(
                to_string(&SubmitPayForBlobResponse {
                    height: 100,
                    rest: Value::Null,
                })
                .unwrap(),
            )
            .unwrap();
            if let CelestiaMode::Delayed(n) = self.mode {
                tokio::time::sleep(Duration::from_millis(n * self.sequencer_block_time_ms)).await;
            }

            Ok(rsp)
        }
    }

    // the mock sequencer server
    #[derive(Deserialize)]
    struct ProxyQuery {
        query: String,
    }

    #[derive(Deserialize)]
    #[serde(try_from = "ProxyQuery")]
    #[allow(unreachable_pub)]
    pub struct Query {
        _query: tendermint_rpc::query::Query,
    }

    impl TryFrom<ProxyQuery> for Query {
        type Error = tendermint_rpc::error::Error;

        fn try_from(proxy: ProxyQuery) -> Result<Self, Self::Error> {
            let query = proxy.query.parse::<tendermint_rpc::query::Query>()?;
            Ok(Self {
                _query: query,
            })
        }
    }

    #[derive(Debug)]
    struct RandomU256IdProvider;

    impl IdProvider for RandomU256IdProvider {
        fn next_id(&self) -> SubscriptionId<'static> {
            use ethers::types::U256;
            use impl_serde::serialize::to_hex;
            use rand::RngCore as _;

            let mut rng = rand::thread_rng();
            let mut raw_u256 = [0u8; 32];
            rng.fill_bytes(&mut raw_u256);
            // Just in case, convert to u256 and back to big endian because parity's u256
            // implementation does some extra complex transformations.
            let u256 = U256::from(raw_u256);
            let mut byte_repr = [0u8; 32];
            u256.to_big_endian(&mut byte_repr);
            let u256_ser = to_hex(&byte_repr, true);
            SubscriptionId::from(u256_ser)
        }
    }

    #[rpc(server)]
    trait Sequencer {
        #[subscription(name = "subscribe", item = Query)]
        async fn subscribe(&self, queury: Query) -> SubscriptionResult;
    }

    struct SequencerImpl {
        new_tx_sender: Sender<Transaction>,
    }

    #[async_trait]
    impl SequencerServer for SequencerImpl {
        async fn subscribe(
            &self,
            pending: PendingSubscriptionSink,
            _query: Query,
        ) -> SubscriptionResult {
            use jsonrpsee::server::SubscriptionMessage;
            let sink = pending.accept().await?;
            let mut rx = self.new_tx_sender.subscribe();
            loop {
                tokio::select!(
                    biased;
                    () = sink.closed() => break,
                    Ok(new_tx) = rx.recv() => sink.send(
                        SubscriptionMessage::from_json(&new_tx)?
                    ).await?,
                );
            }
            Ok(())
        }
    }

    pub(crate) struct MockSequencer {
        /// The local address to which the mocked jsonrpc server is bound.
        local_addr: String,
        _server_task_handle: tokio::task::JoinHandle<()>,
    }

    impl MockSequencer {
        /// Spawns a new mocked sequencer server.
        /// # Panics
        /// Panics if the server fails to start.
        pub(crate) async fn spawn() -> Self {
            use jsonrpsee::server::Server;
            let server = Server::builder()
                .ws_only()
                .set_id_provider(RandomU256IdProvider)
                .build("127.0.0.1:0")
                .await
                .expect("should be able to start a jsonrpsee server bound to a 0 port");
            let local_addr = server
                .local_addr()
                .expect("server should have a local addr");
            let (new_tx_sender, _) = channel(256);
            let mock_geth_impl = SequencerImpl {
                new_tx_sender,
            };
            let handle = server.start(mock_geth_impl.into_rpc());
            let server_task_handle = tokio::spawn(handle.stopped());
            Self {
                local_addr: format!("ws://{}", local_addr),
                _server_task_handle: server_task_handle,
            }
        }

        #[must_use]
        pub(crate) fn local_addr(&self) -> String {
            self.local_addr.clone()
        }
    }
}
