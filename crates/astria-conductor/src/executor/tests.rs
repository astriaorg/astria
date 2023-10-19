use std::net::SocketAddr;

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

#[derive(Debug)]
struct MockExecutionServer {
    _server_handle: JoinHandle<()>,
    local_addr: SocketAddr,
}

impl MockExecutionServer {
    async fn spawn() -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(ExecutionServiceServer::new(ExecutionServiceImpl))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await;
        });
        Self {
            _server_handle: server_handle,
            local_addr,
        }
    }

    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

struct ExecutionServiceImpl;

#[tonic::async_trait]
impl ExecutionService for ExecutionServiceImpl {
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

struct MockEnvironment {
    _server: MockExecutionServer,
    _block_tx: super::Sender,
    _shutdown_tx: oneshot::Sender<()>,
    executor: Executor,
}

async fn start_mock(disable_empty_block_execution: bool) -> MockEnvironment {
    let _server = MockExecutionServer::spawn().await;
    let chain_id = ChainId::new(b"test".to_vec()).unwrap();
    let server_url = format!("http://{}", _server.local_addr());

    let (_block_tx, block_rx) = mpsc::unbounded_channel();
    let (_shutdown_tx, shutdown_rx) = oneshot::channel();
    let executor = Executor::new(
        &server_url,
        chain_id,
        disable_empty_block_execution,
        block_rx,
        shutdown_rx,
        None,
    )
    .await
    .unwrap();

    MockEnvironment {
        _server,
        _block_tx,
        _shutdown_tx,
        executor,
    }
}

#[tokio::test]
async fn execute_sequencer_block_without_txs() {
    let mut mock = start_mock(false).await;

    let expected_exection_hash = hash(&mock.executor.execution_state);
    let block = get_test_block_subset();

    let execution_block_hash = mock
        .executor
        .execute_block(block)
        .await
        .unwrap()
        .expect("expected execution block hash");
    assert_eq!(expected_exection_hash, execution_block_hash);
}

#[tokio::test]
async fn skip_sequencer_block_without_txs() {
    let mut mock = start_mock(true).await;

    let block = get_test_block_subset();
    let execution_block_hash = mock.executor.execute_block(block).await.unwrap();
    assert!(execution_block_hash.is_none());
}

#[tokio::test]
async fn execute_unexecuted_da_block_with_transactions() {
    let mut mock = start_mock(false).await;

    let mut block = get_test_block_subset();
    block.rollup_transactions.push(b"test_transaction".to_vec());

    let expected_exection_hash = hash(&mock.executor.execution_state);

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(expected_exection_hash, mock.executor.execution_state);
    // should be empty because 1 block was executed and finalized, which
    // deletes it from the map
    assert!(mock.executor.sequencer_hash_to_execution_hash.is_empty());
}

#[tokio::test]
async fn skip_unexecuted_da_block_with_no_transactions() {
    let mut mock = start_mock(true).await;

    let block = get_test_block_subset();
    let previous_execution_state = mock.executor.execution_state.clone();

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(previous_execution_state, mock.executor.execution_state,);
    // should be empty because nothing was executed
    assert!(mock.executor.sequencer_hash_to_execution_hash.is_empty());
}

#[tokio::test]
async fn execute_unexecuted_da_block_with_no_transactions() {
    let mut mock = start_mock(false).await;
    let block = get_test_block_subset();
    let expected_execution_state = hash(&mock.executor.execution_state);

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(expected_execution_state, mock.executor.execution_state);
    // should be empty because block was executed and finalized, which
    // deletes it from the map
    assert!(mock.executor.sequencer_hash_to_execution_hash.is_empty());
}

#[tokio::test]
async fn empty_message_from_data_availability_is_dropped() {
    let mut mock = start_mock(false).await;
    let expected_execution_state = mock.executor.execution_state.clone();

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![])
        .await
        .unwrap();

    assert_eq!(expected_execution_state, mock.executor.execution_state);
    assert!(mock.executor.sequencer_hash_to_execution_hash.is_empty());
}
