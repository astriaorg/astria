use std::collections::HashMap;

use astria_sequencer_types::{
    ChainId,
    SequencerBlockData,
};
use color_eyre::eyre::{
    self,
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use tendermint::{
    Hash,
    Time,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
        oneshot,
    },
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    execution_client::{
        ExecutionClient,
        ExecutionRpcClient,
    },
    types::SequencerBlockSubset,
};

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

// Given `Time`, convert to protobuf timestamp
fn convert_tendermint_to_prost_timestamp(value: Time) -> Result<ProstTimestamp> {
    use tendermint_proto::google::protobuf::Timestamp as TendermintTimestamp;
    let TendermintTimestamp {
        seconds,
        nanos,
    } = value.into();
    Ok(ProstTimestamp {
        seconds,
        nanos,
    })
}

#[derive(Debug)]
pub(crate) enum ExecutorCommand {
    /// used when a block is received from the subscription stream to sequencer
    BlockReceivedFromSequencer { block: Box<SequencerBlockData> },
    /// used when a block is received from the reader (Celestia)
    FromCelestia(Vec<SequencerBlockSubset>),
}

pub(crate) struct Executor {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,

    shutdown: oneshot::Receiver<()>,

    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionRpcClient,

    /// Chain ID
    chain_id: ChainId,

    /// Tracks the state of the execution chain
    execution_state: Vec<u8>,

    /// map of sequencer block hash to execution block hash
    ///
    /// this is required because when we receive sequencer blocks (from network or DA),
    /// we only know the sequencer block hash, but not the execution block hash,
    /// as the execution block hash is created by executing the block.
    /// as well, the execution layer is not aware of the sequencer block hash.
    /// we need to track the mapping of sequencer block hash -> execution block hash
    /// so that we can mark the block as final on the execution layer when
    /// we receive a finalized sequencer block.
    sequencer_hash_to_execution_hash: HashMap<Hash, Vec<u8>>,

    /// Chose to execute empty blocks or not
    disable_empty_block_execution: bool,
}

impl Executor {
    pub(crate) async fn new(
        client_url: &str,
        chain_id: ChainId,
        disable_empty_block_execution: bool,
        cmd_rx: Receiver,
        shutdown: oneshot::Receiver<()>,
    ) -> Result<Self> {
        let mut execution_rpc_client = ExecutionRpcClient::new(client_url)
            .await
            .wrap_err("failed to create execution rpc client")?;
        let init_state_response = execution_rpc_client
            .call_init_state()
            .await
            .wrap_err("could not initialize execution rpc client state")?;
        let execution_state = init_state_response.block_hash;
        Ok(Self {
            cmd_rx,
            shutdown,
            execution_rpc_client,
            chain_id,
            execution_state,
            sequencer_hash_to_execution_hash: HashMap::new(),
            disable_empty_block_execution,
        })
    }

    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        loop {
            select!(
                biased;

                shutdown = &mut self.shutdown => {
                    match shutdown {
                        Err(e) => warn!(error.message = %e, "shutdown channel return with error; shutting down"),
                        Ok(()) => info!("received shutdown signal; shutting down"),
                    }
                    break;
                }

                cmd = self.cmd_rx.recv() => {
                    let Some(cmd) = cmd else {
                        error!("cmd channel closed unexpectedly; shutting down");
                        break;
                    };
                    match cmd {
                        ExecutorCommand::BlockReceivedFromSequencer {
                            block,
                        } => {
                            let height = block.header().height.value();
                            let block_subset =
                                SequencerBlockSubset::from_sequencer_block_data(*block, &self.chain_id);

                            if let Err(e) = self.execute_block(block_subset).await {
                                error!(
                                    height = height,
                                    error = ?e,
                                    "failed to execute block"
                                );
                            }
                        }

                        ExecutorCommand::FromCelestia(mut subsets) => {
                            // FIXME: actually process all the blocks
                            let block = subsets.remove(0);
                            let height = block.header.height.value();
                            if let Err(e) = self
                                .handle_block_received_from_data_availability(block)
                                .await
                            {
                                error!(
                                    height = height,
                                    error = ?e,
                                    "failed to finalize block"
                                );
                            }
                        }
                    }
                }
            )
        }
        Ok(())
    }

    /// checks for relevant transactions in the SequencerBlock and attempts
    /// to execute them via the execution service function DoBlock.
    /// if there are relevant transactions that successfully execute,
    /// it returns the resulting execution block hash.
    /// if the block has already been executed, it returns the previously-computed
    /// execution block hash.
    /// if there are no relevant transactions in the SequencerBlock, it returns None.
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Option<Vec<u8>>> {
        if self.disable_empty_block_execution && block.rollup_transactions.is_empty() {
            debug!(
                height = block.header.height.value(),
                "no transactions in block, skipping execution"
            );
            return Ok(None);
        }

        if let Some(execution_hash) = self.sequencer_hash_to_execution_hash.get(&block.block_hash) {
            debug!(
                height = block.header.height.value(),
                execution_hash = hex::encode(execution_hash),
                "block already executed"
            );
            return Ok(Some(execution_hash.clone()));
        }

        let prev_block_hash = self.execution_state.clone();
        info!(
            height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let response = self
            .execution_rpc_client
            .call_do_block(prev_block_hash, block.rollup_transactions, Some(timestamp))
            .await?;
        self.execution_state = response.block_hash.clone();

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&response.block_hash),
            "executed sequencer block",
        );
        self.sequencer_hash_to_execution_hash
            .insert(block.block_hash, response.block_hash.clone());

        Ok(Some(response.block_hash))
    }

    async fn handle_block_received_from_data_availability(
        &mut self,
        block: SequencerBlockSubset,
    ) -> Result<()> {
        let sequencer_block_hash = block.block_hash;
        let maybe_execution_block_hash = self
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_execution_block_hash {
            Some(execution_block_hash) => {
                self.finalize_block(execution_block_hash, sequencer_block_hash)
                    .await?;
            }
            None => {
                // this means either:
                // - we didn't receive the block from the sequencer stream, or
                // - we received it, but the sequencer block didn't contain
                // any transactions for this rollup namespace, thus nothing was executed
                // on receiving this block.

                // try executing the block as it hasn't been executed before
                // execute_block will check if our namespace has txs; if so, it'll return the
                // resulting execution block hash, otherwise None
                let Some(execution_block_hash) = self
                    .execute_block(block)
                    .await
                    .wrap_err("failed to execute block")?
                else {
                    // no txs for our namespace, nothing to do
                    debug!("execute_block returned None; skipping finalize_block");
                    return Ok(());
                };

                // finalize the block after it's been executed
                self.finalize_block(execution_block_hash, sequencer_block_hash)
                    .await?;
            }
        };
        Ok(())
    }

    /// This function finalizes the given execution block on the execution layer by calling
    /// the execution service's FinalizeBlock function.
    /// note that this function clears the respective entry in the
    /// `sequencer_hash_to_execution_hash` map.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - the call to the execution service's FinalizeBlock function fails
    #[instrument(ret, err, skip_all, fields(
        execution_block_hash = hex::encode(&execution_block_hash),
        sequencer_block_hash = hex::encode(sequencer_block_hash),
    ))]
    async fn finalize_block(
        &mut self,
        execution_block_hash: Vec<u8>,
        sequencer_block_hash: Hash,
    ) -> Result<()> {
        self.execution_rpc_client
            .call_finalize_block(execution_block_hash)
            .await
            .wrap_err("failed to finalize block")?;
        self.sequencer_hash_to_execution_hash
            .remove(&sequencer_block_hash);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;

    use color_eyre::eyre::Result;
    use prost_types::Timestamp;
    use proto::generated::execution::v1alpha1::{
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
    use sha2::Digest as _;
    use tokio::{
        sync::{
            mpsc,
            oneshot,
        },
        task::JoinHandle,
    };
    use tonic::transport::Server;

    use super::*;

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
            let prev_block_hash = _request.into_inner().prev_block_hash;
            let res = hash(&prev_block_hash);
            Ok(tonic::Response::new(DoBlockResponse {
                block_hash: res.to_vec(),
            }))
        }

        async fn finalize_block(
            &self,
            _request: tonic::Request<FinalizeBlockRequest>,
        ) -> std::result::Result<tonic::Response<FinalizeBlockResponse>, tonic::Status> {
            Ok(tonic::Response::new(FinalizeBlockResponse {}))
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

    fn hash(s: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(s);
        hasher.finalize().to_vec()
    }

    fn get_test_block_subset() -> SequencerBlockSubset {
        SequencerBlockSubset {
            block_hash: hash(b"block1").try_into().unwrap(),
            header: astria_sequencer_types::test_utils::default_header(),
            rollup_transactions: vec![],
        }
    }

    fn get_test_config() -> crate::Config {
        crate::Config {
            chain_id: "test".to_string(),
            execution_rpc_url: "test".to_string(),
            disable_finalization: false,
            log: "test".to_string(),
            disable_empty_block_execution: false,
            celestia_node_url: "test".to_string(),
            celestia_bearer_token: "test".to_string(),
            sequencer_url: "test".to_string(),
            initial_sequencer_block_height: 1,
        }
    }

    #[tokio::test]
    async fn execute_sequencer_block_without_txs() {
        let mut cfg = get_test_config();
        let chain_id = ChainId::new(cfg.chain_id.as_bytes().to_vec()).unwrap();

        let execution_server = MockExecution::spawn().await;
        cfg.execution_rpc_url = execution_server.local_addr();

        let (_block_tx, block_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut executor = Executor::new(
            &cfg.execution_rpc_url,
            chain_id,
            cfg.disable_empty_block_execution,
            block_rx,
            shutdown_rx,
        )
        .await
        .unwrap();

        let expected_exection_hash = hash(&executor.execution_state);
        let block = get_test_block_subset();

        let execution_block_hash = executor
            .execute_block(block)
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
    }

    #[tokio::test]
    async fn skip_sequencer_block_without_txs() {
        let mut cfg = get_test_config();
        let chain_id = ChainId::new(cfg.chain_id.as_bytes().to_vec()).unwrap();
        cfg.disable_empty_block_execution = true;
        let execution_server = MockExecution::spawn().await;
        cfg.execution_rpc_url = execution_server.local_addr();

        let (_block_tx, block_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut executor = Executor::new(
            &cfg.execution_rpc_url,
            chain_id,
            cfg.disable_empty_block_execution,
            block_rx,
            shutdown_rx,
        )
        .await
        .unwrap();

        let block = get_test_block_subset();
        let execution_block_hash = executor.execute_block(block).await.unwrap();
        assert!(execution_block_hash.is_none());
    }

    #[tokio::test]
    async fn execute_unexecuted_da_block_with_transactions() {
        let mut cfg = get_test_config();
        let chain_id = ChainId::new(cfg.chain_id.as_bytes().to_vec()).unwrap();

        let execution_server = MockExecution::spawn().await;
        cfg.execution_rpc_url = execution_server.local_addr();

        let (_block_tx, block_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut executor = Executor::new(
            &cfg.execution_rpc_url,
            chain_id,
            cfg.disable_empty_block_execution,
            block_rx,
            shutdown_rx,
        )
        .await
        .unwrap();

        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let expected_exection_hash = hash(&executor.execution_state);

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        assert_eq!(expected_exection_hash, executor.execution_state);
        // should be empty because 1 block was executed and finalized, which
        // deletes it from the map
        assert!(executor.sequencer_hash_to_execution_hash.is_empty());
    }

    #[tokio::test]
    async fn skip_unexecuted_da_block_with_no_transactions() {
        let mut cfg = get_test_config();
        let chain_id = ChainId::new(cfg.chain_id.as_bytes().to_vec()).unwrap();
        cfg.disable_empty_block_execution = true;

        let execution_server = MockExecution::spawn().await;
        cfg.execution_rpc_url = execution_server.local_addr();

        let (_block_tx, block_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut executor = Executor::new(
            &cfg.execution_rpc_url,
            chain_id,
            cfg.disable_empty_block_execution,
            block_rx,
            shutdown_rx,
        )
        .await
        .unwrap();

        let block: SequencerBlockSubset = get_test_block_subset();
        let previous_execution_state = executor.execution_state.clone();

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        assert_eq!(previous_execution_state, executor.execution_state,);
        // should be empty because nothing was executed
        assert!(executor.sequencer_hash_to_execution_hash.is_empty());
    }

    #[tokio::test]
    async fn execute_unexecuted_da_block_with_no_transactions() {
        let mut cfg = get_test_config();
        let chain_id = ChainId::new(cfg.chain_id.as_bytes().to_vec()).unwrap();

        let execution_server = MockExecution::spawn().await;
        cfg.execution_rpc_url = execution_server.local_addr();

        let (_block_tx, block_rx) = mpsc::unbounded_channel();
        let (_shutdown_tx, shutdown_rx) = oneshot::channel();
        let mut executor = Executor::new(
            &cfg.execution_rpc_url,
            chain_id,
            cfg.disable_empty_block_execution,
            block_rx,
            shutdown_rx,
        )
        .await
        .unwrap();

        let block: SequencerBlockSubset = get_test_block_subset();
        let expected_execution_state = hash(&executor.execution_state);

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        assert_eq!(expected_execution_state, executor.execution_state);
        // should be empty because block was executed and finalized, which
        // deletes it from the map
        assert!(executor.sequencer_hash_to_execution_hash.is_empty());
    }
}
