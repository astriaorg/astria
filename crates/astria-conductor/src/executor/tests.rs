use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_optimism::test_utils::deploy_mock_optimism_portal;
use ethers::{
    prelude::*,
    utils::AnvilInstance,
};
use k256::ecdsa::SigningKey;
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
        let hash = get_expected_execution_hash(&parent_block_hash, &loc_request.transactions);
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

fn get_expected_execution_hash(parent_block_hash: &[u8], transactions: &[Vec<u8>]) -> Vec<u8> {
    hash(&[parent_block_hash, &transactions.concat()].concat())
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

async fn start_mock(
    disable_empty_block_execution: bool,
    pre_execution_hook: Option<Box<dyn PreExecutionHook>>,
) -> MockEnvironment {
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
        pre_execution_hook,
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

async fn start_mock_with_optimism_handler() -> (
    MockEnvironment,
    Address,
    Arc<Provider<Ws>>,
    Wallet<SigningKey>,
    AnvilInstance,
) {
    let (contract_address, provider, wallet, anvil) = deploy_mock_optimism_portal().await;

    let pre_execution_hook: Option<Box<dyn PreExecutionHook>> = Some(Box::new(
        crate::executor::optimism::Handler::new(provider.clone(), contract_address, 1).await,
    ));
    (
        start_mock(false, pre_execution_hook).await,
        contract_address,
        provider,
        wallet,
        anvil,
    )
}

#[tokio::test]
async fn execute_sequencer_block_without_txs() {
    let mut mock = start_mock(false, None).await;

    // using soft hash here as sequencer blocks are executed on top of the soft commitment
    let expected_exection_hash =
        get_expected_execution_hash(&mock.executor.commitment_state.soft.hash, &[]);
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
    let mut mock = start_mock(true, None).await;

    let block = get_test_block_subset();
    let execution_block_hash = mock.executor.execute_block(block).await.unwrap();
    assert!(execution_block_hash.is_none());
}

#[tokio::test]
async fn execute_unexecuted_da_block_with_transactions() {
    let mut mock = start_mock(false, None).await;

    let mut block = get_test_block_subset();
    block.rollup_transactions.push(b"test_transaction".to_vec());

    // using firm hash here as da blocks are executed on top of the firm commitment
    let expected_exection_hash = get_expected_execution_hash(
        &mock.executor.commitment_state.firm.hash,
        &block.rollup_transactions,
    );

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
    let mut mock = start_mock(true, None).await;

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
    let mut mock = start_mock(false, None).await;
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
    let mut mock = start_mock(false, None).await;
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
async fn deposit_events_are_converted_and_executed() {
    use astria_optimism::contract::*;

    // make a deposit transaction
    let (mut mock, contract_address, provider, wallet, _anvil) =
        start_mock_with_optimism_handler().await;
    let contract = get_optimism_portal_with_signer(provider.clone(), wallet, contract_address);
    let to = Address::zero();
    let value = U256::from(100);
    let receipt = make_deposit_transaction(&contract, Some(to), value, None)
        .await
        .unwrap()
        .unwrap();
    assert!(receipt.status.unwrap().as_u64() == 1);

    // get the event and the expected deposit transaction
    let to_block = provider.get_block_number().await.unwrap();
    let event_filter = contract
        .event::<TransactionDepositedFilter>()
        .from_block(1)
        .to_block(to_block);

    let events = event_filter.query_with_meta().await.unwrap();

    let deposit_txs =
        crate::executor::optimism::convert_deposit_events_to_encoded_txs(events).unwrap();

    // calculate the expected mock execution hash, which includes the block txs,
    // thus confirming the deposit tx was executed
    let expected_exection_hash =
        get_expected_execution_hash(&mock.executor.commitment_state.soft.hash, &deposit_txs);
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
