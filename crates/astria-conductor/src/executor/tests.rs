use std::net::SocketAddr;

use proto::generated::execution::v1alpha2::{
    execution_service_server::{
        ExecutionService,
        ExecutionServiceServer,
    },
    BatchGetBlocksRequest,
    BatchGetBlocksResponse,
    Block,
    CommitmentState,
    ExecuteBlockRequest,
    GetBlockRequest,
    GetCommitmentStateRequest,
    UpdateCommitmentStateRequest,
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

fn new_basic_block() -> Block {
    Block {
        number: 0,
        hash: vec![],
        parent_block_hash: vec![],
        timestamp: None,
    }
}

struct ExecutionServiceImpl;

#[tonic::async_trait]
impl ExecutionService for ExecutionServiceImpl {
    async fn get_block(
        &self,
        _request: tonic::Request<GetBlockRequest>,
    ) -> std::result::Result<tonic::Response<Block>, tonic::Status> {
        unimplemented!("get_block")
    }

    async fn batch_get_blocks(
        &self,
        _request: tonic::Request<BatchGetBlocksRequest>,
    ) -> std::result::Result<tonic::Response<BatchGetBlocksResponse>, tonic::Status> {
        unimplemented!("batch_get_blocks")
    }

    async fn execute_block(
        &self,
        request: tonic::Request<ExecuteBlockRequest>,
    ) -> std::result::Result<tonic::Response<Block>, tonic::Status> {
        let loc_request = request.into_inner();
        let parent_block_hash = loc_request.prev_block_hash.clone();
        let hash = hash(&parent_block_hash);
        let timestamp = loc_request.timestamp.unwrap_or_default();
        Ok(tonic::Response::new(Block {
            number: 0, // we don't do anything with the number in the mock, can always be 0
            hash,
            parent_block_hash,
            timestamp: Some(timestamp),
        }))
    }

    async fn get_commitment_state(
        &self,
        _request: tonic::Request<GetCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<CommitmentState>, tonic::Status> {
        Ok(tonic::Response::new(CommitmentState {
            soft: Some(new_basic_block()),
            firm: Some(new_basic_block()),
        }))
    }

    async fn update_commitment_state(
        &self,
        request: tonic::Request<UpdateCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<CommitmentState>, tonic::Status> {
        Ok(tonic::Response::new(
            request.into_inner().commitment_state.unwrap(),
        ))
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
        1, // genesis block is always block 0, first executable block will always be block 1
        block_rx,
        shutdown_rx,
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

    // using soft hash here as sequencer blocks are executed on top of the soft commitment
    let expected_exection_hash = hash(&mock.executor.commitment_state.soft.hash);
    let block = get_test_block_subset();

    let execution_block_hash = mock
        .executor
        .execute_block(block)
        .await
        .unwrap()
        .expect("expected execution block hash")
        .hash;
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

    // using firm hash here as da blocks are executed on top of the firm commitment
    let expected_exection_hash = hash(&mock.executor.commitment_state.firm.hash);

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(
        expected_exection_hash,
        mock.executor.commitment_state.firm.hash
    );
    // should be empty because 1 block was executed and finalized, which
    // deletes it from the map
    assert!(mock.executor.sequencer_hash_to_execution_block.is_empty());
}

#[tokio::test]
async fn skip_unexecuted_da_block_with_no_transactions() {
    let mut mock = start_mock(true).await;

    let block = get_test_block_subset();
    // using firm hash here as da blocks are executed on top of the firm commitment
    let previous_execution_state = mock.executor.commitment_state.firm.hash.clone();

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(
        previous_execution_state,
        mock.executor.commitment_state.firm.hash,
    );
    // should be empty because nothing was executed
    assert!(mock.executor.sequencer_hash_to_execution_block.is_empty());
}

#[tokio::test]
async fn execute_unexecuted_da_block_with_no_transactions() {
    let mut mock = start_mock(false).await;
    let block = get_test_block_subset();
    // using firm hash here as da blocks are executed on top of the firm commitment
    let expected_execution_state = hash(&mock.executor.commitment_state.firm.hash);

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await
        .unwrap();

    assert_eq!(
        expected_execution_state,
        mock.executor.commitment_state.firm.hash
    );
    // should be empty because block was executed and finalized, which
    // deletes it from the map
    assert!(mock.executor.sequencer_hash_to_execution_block.is_empty());
}

#[tokio::test]
async fn empty_message_from_data_availability_is_dropped() {
    let mut mock = start_mock(false).await;
    // using firm hash here as da blocks are executed on top of the firm commitment
    let expected_execution_state = mock.executor.commitment_state.firm.hash.clone();

    mock.executor
        .execute_and_finalize_blocks_from_celestia(vec![])
        .await
        .unwrap();

    assert_eq!(
        expected_execution_state,
        mock.executor.commitment_state.firm.hash
    );
    assert!(mock.executor.sequencer_hash_to_execution_block.is_empty());
}

#[tokio::test]
async fn try_execute_out_of_order_block_from_sequencer() {
    let mut mock = start_mock(false).await;
    let mut block = get_test_block_subset();

    // 0 is always the genesis block, so this should fail
    block.header.height = 0_u32.into();
    let execution_result = mock.executor.execute_block(block.clone()).await;
    assert!(execution_result.is_err());

    // the first block to execute should always be 1, this should fail as it is
    // in the future
    block.header.height = 2_u32.into();
    let execution_result = mock.executor.execute_block(block).await;
    assert!(execution_result.is_err());
}

#[tokio::test]
async fn try_execute_out_of_order_block_from_celestia() {
    let mut mock = start_mock(false).await;
    let mut block = get_test_block_subset();

    // 0 is always the genesis block, so this should fail
    block.header.height = 0_u32.into();
    let execution_result = mock
        .executor
        .execute_and_finalize_blocks_from_celestia(vec![block.clone()])
        .await;
    assert!(execution_result.is_err());

    // the first block to execute should always be 1, this should fail as it is
    // in the future
    block.header.height = 2_u32.into();
    let execution_result = mock
        .executor
        .execute_and_finalize_blocks_from_celestia(vec![block])
        .await;
    assert!(execution_result.is_err());
}
